//! V2 native Windows shell: no .NET, Chromium, WebView, or local HTTP server.

use std::{f64::consts::PI, fs, mem::{size_of, MaybeUninit}, path::PathBuf, sync::{Mutex, OnceLock}, thread, time::{Duration, SystemTime, UNIX_EPOCH}};
use r5_battery_estimator::{battery::transport::R5HidTransport, probe_once, ProbeResult};
use serde::{Deserialize, Serialize};
use windows::{
    core::w,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{Arc, BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, InvalidateRect, LineTo, MoveToEx, PAINTSTRUCT, RoundRect, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_LEFT, DT_SINGLELINE, DT_TOP, FW_BOLD, FW_NORMAL, OUT_DEFAULT_PRECIS, PS_SOLID, TRANSPARENT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::{Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE}, WindowsAndMessaging::{AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, LoadCursorW, LoadIconW, PostQuitMessage, RegisterClassW, SendMessageW, SetForegroundWindow, SetTimer, ShowWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HTCAPTION, IDC_ARROW, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_POPUP, WS_VISIBLE}},
    },
};

const CLASS: windows::core::PCWSTR = w!("R5BatteryEstimatorNativeV2");
const TRAY_MESSAGE: u32 = WM_APP + 17;
static BATTERY: OnceLock<Mutex<ProbeResult>> = OnceLock::new();
static HISTORY: OnceLock<Mutex<Vec<Sample>>> = OnceLock::new();

#[derive(Clone, Serialize, Deserialize)]
struct Sample { at: u64, percent: u8, charging: bool }

fn main() -> windows::core::Result<()> {
    unsafe {
        let initial = probe_once(&R5HidTransport::new());
        let state = BATTERY.get_or_init(|| Mutex::new(initial.clone()));
        HISTORY.get_or_init(|| Mutex::new(load_history()));
        record_sample(&initial);
        let state = state as &'static Mutex<ProbeResult>;
        thread::spawn(move || loop { thread::sleep(Duration::from_secs(30)); let result = probe_once(&R5HidTransport::new()); *state.lock().expect("battery state poisoned") = result.clone(); record_sample(&result); });
        let instance = GetModuleHandleW(None)?;
        let cursor = LoadCursorW(None, IDC_ARROW)?;
        let class = WNDCLASSW { hCursor: cursor, hInstance: instance.into(), lpszClassName: CLASS, style: CS_HREDRAW | CS_VREDRAW, lpfnWndProc: Some(window_proc), ..Default::default() };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(WINDOW_EX_STYLE::default(), CLASS, w!("R5 Battery Estimator V2"), WS_POPUP | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, 1140, 950, None, None, Some(instance.into()), None)?;
        add_tray(hwnd)?;
        let _ = SetTimer(Some(hwnd), 1, 30_000, None);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let mut message = MaybeUninit::<MSG>::zeroed();
        while GetMessageW(message.as_mut_ptr(), None, 0, 0).into() {
            let message = message.assume_init();
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        TRAY_MESSAGE if lparam.0 as u32 == WM_LBUTTONUP => { let _ = ShowWindow(hwnd, SW_SHOW); LRESULT(0) }
        TRAY_MESSAGE if lparam.0 as u32 == WM_RBUTTONUP => { show_tray_menu(hwnd); LRESULT(0) }
        WM_COMMAND => match (wparam.0 & 0xffff) as usize { 1 => { let _ = ShowWindow(hwnd, SW_SHOW); LRESULT(0) }, 2 => { if let Some(state) = BATTERY.get() { let result = probe_once(&R5HidTransport::new()); *state.lock().expect("battery state poisoned") = result.clone(); record_sample(&result); } let _ = InvalidateRect(Some(hwnd), None, false); LRESULT(0) }, 3 => { let _ = DestroyWindow(hwnd); LRESULT(0) }, _ => LRESULT(0) },
        WM_CLOSE => { let _ = ShowWindow(hwnd, SW_HIDE); LRESULT(0) }
        WM_LBUTTONDOWN => { let x = (lparam.0 & 0xffff) as i32; let y = ((lparam.0 >> 16) & 0xffff) as i32; if y < 50 && x > 1070 { let _ = ShowWindow(hwnd, SW_HIDE); } else if y < 50 && x > 1010 { let _ = ShowWindow(hwnd, SW_HIDE); } else if y > 872 && x > 852 { if let Some(state) = BATTERY.get() { *state.lock().expect("battery state poisoned") = probe_once(&R5HidTransport::new()); } let _ = InvalidateRect(Some(hwnd), None, false); } else if y < 50 { let _ = SendMessageW(hwnd, WM_NCLBUTTONDOWN, Some(WPARAM(HTCAPTION as usize)), Some(LPARAM(0))); } LRESULT(0) }
        WM_TIMER => { let _ = InvalidateRect(Some(hwnd), None, false); LRESULT(0) }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            let background = CreateSolidBrush(COLORREF(0x17110A));
            FillRect(dc, &paint.rcPaint, background);
            let _ = DeleteObject(background.into());
            let header = CreateSolidBrush(COLORREF(0x1D160D)); FillRect(dc, &windows::Win32::Foundation::RECT { left: 0, top: 0, right: 1140, bottom: 50 }, header); let _ = DeleteObject(header.into());
            SetBkMode(dc, TRANSPARENT);
            let (percent, status) = match BATTERY.get().and_then(|s| s.lock().ok()).map(|s| s.clone()) { Some(ProbeResult::Ok { reading }) => (Some(reading.percent), if reading.charging { "Cargando" } else { "No cargando" }), _ => (None, "Sin lectura válida") };
            let samples = HISTORY.get().and_then(|history| history.lock().ok()).map(|items| items.clone()).unwrap_or_default();
            let learned = learned_hours(&samples);
            draw(dc, "R5 Battery Estimator", 60, 70, 700, 70, 42, 0xF5F5F5, true);
            draw(dc, "R5 Battery Estimator", 64, 14, 320, 26, 15, 0xF5F5F5, false); draw(dc, "—", 1028, 12, 24, 26, 18, 0xF5F5F5, false); draw(dc, "×", 1083, 9, 28, 30, 22, 0xF5F5F5, false);
            draw(dc, "Se aprende con tu descarga real.", 62, 135, 700, 42, 18, 0xE9C0B0, false);
            draw(dc, "R5 ULTRA", 915, 85, 160, 45, 17, 0x5353FF, true);
            card(dc, 390, 265, 350, 230); card(dc, 765, 265, 340, 230); chart(dc, &samples); button(dc);
            ring(dc, 58, 198, 305, percent, status);
            draw(dc, "DURACIÓN DE CARGA COMPLETA", 420, 305, 290, 30, 11, 0xE9C0B0, false);
            draw(dc, &learned.map(|hours| format!("{hours:.0} h")).unwrap_or_else(|| "Aprendiendo".to_owned()), 420, 365, 290, 62, 31, 0xF5F5F5, true);
            draw(dc, "Se aprende con tu descarga real.", 420, 445, 290, 30, 11, 0xE9C0B0, false);
            draw(dc, "AUTONOMÍA RESTANTE", 795, 305, 270, 30, 11, 0xE9C0B0, false);
            draw(dc, &percent.map(|value| format!("{:.0} h", learned.unwrap_or(200.0) * value as f64 / 100.0)).unwrap_or_else(|| "—".to_owned()), 795, 365, 270, 70, 48, 0xF5F5F5, true);
            draw(dc, "Se recalcula cada 30 segundos.", 795, 445, 280, 30, 11, 0xE9C0B0, false);
            draw(dc, "La app nunca muestra una desconexión como 0%.", 55, 885, 600, 30, 13, 0xE9C0B0, false);
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_DESTROY => { remove_tray(hwnd); PostQuitMessage(0); LRESULT(0) }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = match CreatePopupMenu() { Ok(menu) => menu, Err(_) => return };
    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Abrir panel"));
    let _ = AppendMenuW(menu, MF_STRING, 2, w!("Actualizar ahora"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, w!(""));
    let _ = AppendMenuW(menu, MF_STRING, 3, w!("Cerrar programa"));
    let mut point = windows::Win32::Foundation::POINT::default();
    if GetCursorPos(&mut point).is_ok() { let _ = SetForegroundWindow(hwnd); let _ = TrackPopupMenu(menu, Default::default(), point.x, point.y, None, hwnd, None); }
    let _ = DestroyMenu(menu);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw(dc: windows::Win32::Graphics::Gdi::HDC, text: &str, left: i32, top: i32, width: i32, height: i32, size: i32, color: u32, bold: bool) {
    let font = CreateFontW(-size, 0, 0, 0, if bold { FW_BOLD.0 as i32 } else { FW_NORMAL.0 as i32 }, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH.0 as u32, w!("Segoe UI"));
    let old = SelectObject(dc, font.into()); SetBkMode(dc, TRANSPARENT); SetTextColor(dc, COLORREF(color));
    let mut wide: Vec<u16> = text.encode_utf16().collect(); let mut rect = windows::Win32::Foundation::RECT { left, top, right: left + width, bottom: top + height }; DrawTextW(dc, &mut wide, &mut rect, DT_LEFT | DT_TOP | DT_SINGLELINE);
    SelectObject(dc, old); let _ = DeleteObject(font.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn card(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, width: i32, height: i32) { let brush = CreateSolidBrush(COLORREF(0x231B10)); let old = SelectObject(dc, brush.into()); let _ = RoundRect(dc, x, y, x + width, y + height, 18, 18); SelectObject(dc, old); let _ = DeleteObject(brush.into()); }

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn chart(dc: windows::Win32::Graphics::Gdi::HDC, samples: &[Sample]) {
    card(dc, 38, 570, 1064, 280); draw(dc, "HISTORIAL: HORA / PORCENTAJE", 85, 605, 420, 30, 15, 0xE9C0B0, true); draw(dc, "100%", 72, 680, 60, 25, 11, 0xE9C0B0, false); draw(dc, "0%", 96, 810, 40, 25, 11, 0xE9C0B0, false); draw(dc, "24 h", 145, 840, 80, 25, 11, 0xE9C0B0, false); draw(dc, "ahora", 1024, 840, 70, 25, 11, 0xE9C0B0, false);
    let grid = CreatePen(PS_SOLID, 1, COLORREF(0x564933)); let old = SelectObject(dc, grid.into()); for row in 0..4 { let y = 690 + row * 34; let _ = MoveToEx(dc, 145, y, None); let _ = LineTo(dc, 1068, y); } for col in 1..4 { let x = 145 + col * 231; let _ = MoveToEx(dc, x, 690, None); let _ = LineTo(dc, x, 824); } SelectObject(dc, old); let _ = DeleteObject(grid.into());
    if samples.len() >= 2 { let green = CreatePen(PS_SOLID, 3, COLORREF(0x87E21A)); let old = SelectObject(dc, green.into()); let first = samples.first().expect("at least two samples"); let last_at = samples.last().expect("at least two samples").at.max(first.at + 1); for (index, sample) in samples.iter().enumerate() { let x = 145 + (((sample.at.saturating_sub(first.at)) as f64 / (last_at - first.at) as f64) * 923.0) as i32; let y = 824 - sample.percent as i32 * 134 / 100; if index == 0 { let _ = MoveToEx(dc, x, y, None); } else { let _ = LineTo(dc, x, y); } } SelectObject(dc, old); let _ = DeleteObject(green.into()); }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn button(dc: windows::Win32::Graphics::Gdi::HDC) { let brush = CreateSolidBrush(COLORREF(0x3D37E8)); let old = SelectObject(dc, brush.into()); let _ = RoundRect(dc, 852, 872, 1080, 932, 12, 12); SelectObject(dc, old); let _ = DeleteObject(brush.into()); draw(dc, "Actualizar ahora", 886, 891, 170, 30, 14, 0xFFFFFF, false); }

fn history_path() -> PathBuf { std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("R5 Battery Estimator").join("rust-v2-history.json") }

fn load_history() -> Vec<Sample> { fs::read_to_string(history_path()).ok().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default() }

fn record_sample(result: &ProbeResult) {
    let ProbeResult::Ok { reading } = result else { return };
    let Some(history) = HISTORY.get() else { return };
    let mut items = history.lock().expect("history poisoned");
    items.push(Sample { at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(), percent: reading.percent, charging: reading.charging });
    let cutoff = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().saturating_sub(14 * 24 * 60 * 60);
    items.retain(|sample| sample.at >= cutoff);
    if let Some(parent) = history_path().parent() { let _ = fs::create_dir_all(parent); }
    if let Ok(raw) = serde_json::to_string(&*items) { let _ = fs::write(history_path(), raw); }
}

fn learned_hours(samples: &[Sample]) -> Option<f64> {
    let start = samples.iter().position(|sample| sample.percent >= 95 && !sample.charging)?;
    if !samples[start..].iter().any(|sample| sample.percent <= 5 && !sample.charging) { return None; }
    let mut elapsed = 0_u64; let mut dropped = 0_u64;
    for pair in samples[start..].windows(2) { let before = &pair[0]; let after = &pair[1]; let gap = after.at.saturating_sub(before.at); if before.charging || after.charging || gap == 0 || gap > 600 || after.percent > before.percent { continue; } elapsed += gap; dropped += (before.percent - after.percent) as u64; }
    if elapsed < 1800 || dropped < 3 { return None; }
    Some((100.0 / (dropped as f64 / (elapsed as f64 / 3600.0))).clamp(5.0, 1000.0))
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn ring(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, size: i32, percent: Option<u8>, status: &str) {
    let left = x + 30; let top = y + 30; let right = x + size - 30; let bottom = y + size - 30; let center_x = (left + right) / 2; let center_y = (top + bottom) / 2; let radius = (right - left) / 2;
    let pen = CreatePen(PS_SOLID, 20, COLORREF(0x2A3948)); let old = SelectObject(dc, pen.into()); let _ = Ellipse(dc, left, top, right, bottom); SelectObject(dc, old); let _ = DeleteObject(pen.into());
    if let Some(percent) = percent.filter(|value| *value > 0) { let end = -PI / 2.0 + (percent as f64 / 100.0) * PI * 2.0; let green = CreatePen(PS_SOLID, 20, COLORREF(0x87E21A)); let old = SelectObject(dc, green.into()); let _ = Arc(dc, left, top, right, bottom, center_x, center_y - radius, center_x + (radius as f64 * end.cos()) as i32, center_y + (radius as f64 * end.sin()) as i32); SelectObject(dc, old); let _ = DeleteObject(green.into()); }
    let value = percent.map(|value| format!("{value}%")).unwrap_or_else(|| "—%".to_owned()); draw(dc, &value, x + 70, y + 100, 180, 70, 48, 0xF5F5F5, true); draw(dc, status, x + 100, y + 205, 150, 30, 13, 0xE9C0B0, false);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn add_tray(hwnd: HWND) -> windows::core::Result<()> {
    let mut data = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP, uCallbackMessage: TRAY_MESSAGE, hIcon: LoadIconW(None, IDI_APPLICATION)?, ..Default::default() };
    let tip: Vec<u16> = "R5 Battery Estimator V2".encode_utf16().collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    Shell_NotifyIconW(NIM_ADD, &data).ok()
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn remove_tray(hwnd: HWND) {
    let data = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
}
