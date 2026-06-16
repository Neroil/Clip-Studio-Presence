use std::{
    ffi::c_void,
    io::Cursor,
    mem::{size_of, zeroed},
};

use reqwest::blocking::{multipart, Client};
use reqwest::header::USER_AGENT;
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

    let url = upload_screenshot(png)?;
    Ok(ShareResult { url })
}

fn upload_screenshot(png: Vec<u8>) -> Result<String, CaptureShareError> {
    upload_to_uguu(png)
}

fn upload_to_uguu(png: Vec<u8>) -> Result<String, CaptureShareError> {
    let part = multipart::Part::bytes(png)
        .file_name("clip-studio-presence.png")
        .mime_str("image/png")?;
    let form = multipart::Form::new().part("files[]", part);

    let client = Client::builder()
        .user_agent("ClipStudioPresence/0.1")
        .http1_only()
        .build()?;

    let response = client
        .post("https://uguu.se/upload?output=text")
        .multipart(form)
        .header(USER_AGENT, "ClipStudioPresence/0.1")
        .send()
        .map_err(|source| CaptureShareError::UploadRequest {
            message: upload_request_message("Uguu", &source),
        })?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|source| CaptureShareError::UploadResponseBody {
            status,
            message: upload_response_body_message("Uguu", status, &source),
        })?;

    if status.is_success() {
        let url = body.trim().to_string();
        if url.starts_with("https://") || url.starts_with("http://") {
            Ok(url)
        } else {
            Err(CaptureShareError::UploadRejected(format!(
                "Uguu replied with HTTP {status}, but the body was not a direct URL. Body text: {url}"
            )))
        }
    } else {
        Err(CaptureShareError::UploadFailed { status, body })
    }
}

fn upload_request_message(host: &str, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return format!(
            "The request timed out before {host} answered. The network may be slow or the host may be blocked."
        );
    }

    if error.is_connect() {
        return format!(
            "The app could not connect to {host}. This is usually a network, DNS, proxy, or TLS problem. Details: {error}"
        );
    }

    if error.is_body() {
        return format!(
            "The request was created, but the upload body could not be sent cleanly to {host}. This usually means the connection was closed while the file was being uploaded. Details: {error}"
        );
    }

    format!("The upload request failed before {host} could answer. Details: {error}")
}

fn upload_response_body_message(host: &str, status: reqwest::StatusCode, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return format!(
            "{host} returned HTTP {status}, but the response body timed out before it could be read."
        );
    }

    if error.is_body() {
        return format!(
            "{host} returned HTTP {status}, but the response body could not be read. This usually means the server closed the connection early. Details: {error}"
        );
    }

    format!(
        "{host} returned HTTP {status}, but reading the response body failed. Details: {error}"
    )
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
    #[error("Could not send the upload request to 0x0.st: {message}")]
    UploadRequest {
        message: String,
    },
    #[error("Uguu returned HTTP {status}, but reading the response body failed: {message}")]
    UploadResponseBody {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("Could not create the upload file part: {0}")]
    Mime(#[from] reqwest::header::InvalidHeaderValue),
    #[error("The image host rejected the upload: {0}")]
    UploadRejected(String),
    #[error("Uguu returned HTTP {status} with body: {body}")]
    UploadFailed {
        status: reqwest::StatusCode,
        body: String,
    },
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
        Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM, RECT},
        Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
            GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, RGBQUAD,
            SRCCOPY,
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

    const CLIP_STUDIO_PROCESS_NAMES: &[&str] = &["CLIPStudioPaint.exe", "CLIPStudioPaintApp.exe"];

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
        bitmap_info.bmiHeader.biSize =
            size_of::<windows_sys::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32;
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
        let end = buffer
            .iter()
            .position(|char| *char == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }
}
