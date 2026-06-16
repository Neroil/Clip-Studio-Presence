#[derive(Clone, Debug, Default)]
pub struct ClipStudioDetection {
    pub running: bool,
    pub focused: bool,
    pub document_title: Option<String>,
}

#[cfg(windows)]
pub fn detect_clip_studio() -> ClipStudioDetection {
    let processes = windows::clip_studio_processes();
    let running = !processes.is_empty();
    let foreground = windows::foreground_window();
    let focused = foreground
        .as_ref()
        .map(|window| processes.iter().any(|process| process.pid == window.pid))
        .unwrap_or(false);

    let document_title = foreground
        .filter(|_| focused)
        .and_then(|window| sanitize_window_title(&window.title));

    ClipStudioDetection {
        running,
        focused,
        document_title,
    }
}

#[cfg(not(windows))]
pub fn detect_clip_studio() -> ClipStudioDetection {
    ClipStudioDetection::default()
}

fn sanitize_window_title(title: &str) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_suffix = trimmed
        .strip_suffix(" - CLIP STUDIO PAINT")
        .or_else(|| trimmed.strip_suffix(" - Clip Studio Paint"))
        .unwrap_or(trimmed)
        .trim();

    if without_suffix.is_empty() {
        None
    } else {
        Some(without_suffix.to_string())
    }
}

#[cfg(windows)]
mod windows {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        },
    };

    const CLIP_STUDIO_PROCESS_NAMES: &[&str] = &["CLIPStudioPaint.exe", "CLIPStudioPaintApp.exe"];

    #[derive(Clone, Debug)]
    pub struct ProcessInfo {
        pub pid: u32,
    }

    #[derive(Clone, Debug)]
    pub struct WindowInfo {
        pub pid: u32,
        pub title: String,
    }

    pub fn clip_studio_processes() -> Vec<ProcessInfo> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = unsafe { zeroed::<PROCESSENTRY32W>() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut processes = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;

        while has_entry {
            let process_name = utf16z_to_string(&entry.szExeFile);
            if is_clip_studio_process(&process_name) {
                processes.push(ProcessInfo {
                    pid: entry.th32ProcessID,
                });
            }
            has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
        }

        unsafe {
            CloseHandle(snapshot);
        }

        processes
    }

    pub fn foreground_window() -> Option<WindowInfo> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return None;
        }

        let title = window_title(hwnd)?;
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }

        if pid == 0 {
            return None;
        }

        Some(WindowInfo { pid, title })
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

    fn is_clip_studio_process(process_name: &str) -> bool {
        CLIP_STUDIO_PROCESS_NAMES
            .iter()
            .any(|candidate| process_name.eq_ignore_ascii_case(candidate))
    }

    fn utf16z_to_string(buffer: &[u16]) -> String {
        let end = buffer
            .iter()
            .position(|char| *char == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }
}
