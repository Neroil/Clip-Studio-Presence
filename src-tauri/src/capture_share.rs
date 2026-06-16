use std::{
    ffi::c_void,
    io::Cursor,
    mem::{size_of, zeroed},
};

use reqwest::blocking::{multipart, Client};
use tauri::{AppHandle, Manager};

pub struct ShareResult {
    pub url: String,
}

pub fn capture_and_upload(app: &AppHandle) -> Result<ShareResult, CaptureShareError> {
    let png = capture_clip_studio_png()?;
    let saved_path = app
        .path()
        .app_cache_dir()
        .map_err(|_| CaptureShareError::AppCacheDir)?
        .join("latest-clip-studio-capture.png");

    if let Some(parent) = saved_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&saved_path, &png)?;

    let url = upload_to_catbox(png)?;
    Ok(ShareResult { url })
}

fn upload_to_catbox(png: Vec<u8>) -> Result<String, CaptureShareError> {
    let part = multipart::Part::bytes(png)
        .file_name("clip-studio-presence.png")
        .mime_str("image/png")?;
    let form = multipart::Form::new()
        .text("reqtype", "fileupload")
        .part("fileToUpload", part);

    let response = Client::new()
        .post("https://catbox.moe/user/api.php")
        .multipart(form)
        .send()?
        .error_for_status()?
        .text()?;

    let url = response.trim().to_string();
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(url)
    } else {
        Err(CaptureShareError::UploadRejected(url))
    }
}

#[cfg(windows)]
fn capture_clip_studio_png() -> Result<Vec<u8>, CaptureShareError> {
    windows_capture::capture_clip_studio_png()
}

#[cfg(not(windows))]
fn capture_clip_studio_png() -> Result<Vec<u8>, CaptureShareError> {
    Err(CaptureShareError::UnsupportedPlatform)
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureShareError {
    #[error("Clip Studio Paint window was not found.")]
    WindowNotFound,
    #[error("Clip Studio Paint window is too small or minimized.")]
    InvalidWindowSize,
    #[error("Could not capture the Clip Studio Paint window.")]
    CaptureFailed,
    #[error("Could not encode the screenshot: {0}")]
    Encode(#[from] png::EncodingError),
    #[error("Could not upload the screenshot: {0}")]
    Upload(#[from] reqwest::Error),
    #[error("Could not create the upload file part: {0}")]
    Mime(#[from] reqwest::header::InvalidHeaderValue),
    #[error("The image host rejected the upload: {0}")]
    UploadRejected(String),
    #[error("Could not access the app cache directory.")]
    AppCacheDir,
    #[error("Could not save the local screenshot: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(not(windows))]
    #[error("Screenshot capture is currently only available on Windows.")]
    UnsupportedPlatform,
}

#[cfg(windows)]
mod windows_capture {
    use super::*;
    use windows_sys::Win32::{
        Foundation::{BOOL, CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM, RECT},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits, GetWindowDC, RGBQUAD, ReleaseDC,
            SRCCOPY, SelectObject,
        },
        Storage::Xps::PrintWindow,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
            GetWindowThreadProcessId, IsWindowVisible,
        },
    };

    const CLIP_STUDIO_PROCESS_NAMES: &[&str] = &[
        "CLIPStudioPaint.exe",
        "CLIPStudioPaintApp.exe",
        "CLIPStudio.exe",
    ];

    pub fn capture_clip_studio_png() -> Result<Vec<u8>, CaptureShareError> {
        let hwnd = find_clip_studio_window()?;
        let mut rect = unsafe { zeroed::<RECT>() };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return Err(CaptureShareError::CaptureFailed);
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 8 || height <= 8 {
            return Err(CaptureShareError::InvalidWindowSize);
        }

        let rgba = capture_window_rgba(hwnd, width, height)?;
        encode_png(width as u32, height as u32, &rgba)
    }

    fn find_clip_studio_window() -> Result<HWND, CaptureShareError> {
        let pids = clip_studio_process_ids();
        if pids.is_empty() {
            return Err(CaptureShareError::WindowNotFound);
        }

        let mut matches = Vec::<HWND>::new();
        let mut context = WindowSearchContext {
            pids: &pids,
            matches: &mut matches,
        };

        unsafe {
            EnumWindows(
                Some(enum_windows_callback),
                (&mut context as *mut WindowSearchContext).cast::<c_void>() as LPARAM,
            );
        }

        matches
            .into_iter()
            .next()
            .ok_or(CaptureShareError::WindowNotFound)
    }

    struct WindowSearchContext<'a> {
        pids: &'a [u32],
        matches: &'a mut Vec<HWND>,
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut WindowSearchContext);
        if IsWindowVisible(hwnd) == 0 || window_title(hwnd).is_none() {
            return 1;
        }

        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if context.pids.contains(&pid) {
            context.matches.push(hwnd);
            return 0;
        }

        1
    }

    fn capture_window_rgba(
        hwnd: HWND,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>, CaptureShareError> {
        let window_dc = unsafe { GetWindowDC(hwnd) };
        if window_dc.is_null() {
            return Err(CaptureShareError::CaptureFailed);
        }

        let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
        if memory_dc.is_null() {
            unsafe {
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureShareError::CaptureFailed);
        }

        let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureShareError::CaptureFailed);
        }

        let old_object = unsafe { SelectObject(memory_dc, bitmap) };
        let printed = unsafe { PrintWindow(hwnd, memory_dc, 2) } != 0;
        if !printed {
            unsafe {
                BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
            }
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: unsafe { zeroed() },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        bitmap_info.bmiHeader.biSize = size_of::<windows_sys::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = width;
        bitmap_info.bmiHeader.biHeight = -height;
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = BI_RGB;

        let mut bgra = vec![0u8; (width * height * 4) as usize];
        let copied = unsafe {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                bgra.as_mut_ptr().cast::<c_void>(),
                &mut bitmap_info,
                DIB_RGB_COLORS,
            )
        };

        unsafe {
            SelectObject(memory_dc, old_object);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
        }

        if copied == 0 {
            return Err(CaptureShareError::CaptureFailed);
        }

        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }

        Ok(bgra)
    }

    fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, CaptureShareError> {
        let mut output = Cursor::new(Vec::new());
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
        drop(writer);
        Ok(output.into_inner())
    }

    fn clip_studio_process_ids() -> Vec<u32> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = unsafe { zeroed::<PROCESSENTRY32W>() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut pids = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;

        while has_entry {
            let process_name = utf16z_to_string(&entry.szExeFile);
            if CLIP_STUDIO_PROCESS_NAMES
                .iter()
                .any(|candidate| process_name.eq_ignore_ascii_case(candidate))
            {
                pids.push(entry.th32ProcessID);
            }
            has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
        }

        unsafe {
            CloseHandle(snapshot);
        }

        pids
    }

    fn window_title(hwnd: HWND) -> Option<String> {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return None;
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..copied as usize]))
    }

    fn utf16z_to_string(buffer: &[u16]) -> String {
        let end = buffer.iter().position(|char| *char == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }
}
