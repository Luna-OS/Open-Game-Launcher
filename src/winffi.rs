//! Direkte Windows-Aufrufe (ohne große Bindings-Crates → kleinere EXE).
//!
//! - `shell_execute`: Programme/URLs/Protokolle öffnen, auch "Als Administrator"
//! - `extract_icon_rgba`: das Original-Icon aus einer EXE holen (256×256)
//! - `set_clipboard`: Text in die Zwischenablage kopieren

#![allow(non_snake_case, dead_code, clippy::upper_case_acronyms)]

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

type HANDLE = *mut c_void;
type BOOL = i32;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[repr(C)]
struct IconInfo {
    f_icon: BOOL,
    x_hotspot: u32,
    y_hotspot: u32,
    hbm_mask: HANDLE,
    hbm_color: HANDLE,
}

#[repr(C)]
struct Bitmap {
    bm_type: i32,
    bm_width: i32,
    bm_height: i32,
    bm_width_bytes: i32,
    bm_planes: u16,
    bm_bits_pixel: u16,
    bm_bits: *mut c_void,
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_ppm: i32,
    y_ppm: i32,
    clr_used: u32,
    clr_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 4],
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: HANDLE,
        op: *const u16,
        file: *const u16,
        params: *const u16,
        dir: *const u16,
        show: i32,
    ) -> HANDLE;
}

#[link(name = "user32")]
extern "system" {
    fn PrivateExtractIconsW(
        file: *const u16,
        index: i32,
        cx: i32,
        cy: i32,
        phicon: *mut HANDLE,
        piconid: *mut u32,
        count: u32,
        flags: u32,
    ) -> u32;
    fn GetIconInfo(hicon: HANDLE, info: *mut IconInfo) -> BOOL;
    fn DestroyIcon(hicon: HANDLE) -> BOOL;
    fn GetDC(hwnd: HANDLE) -> HANDLE;
    fn ReleaseDC(hwnd: HANDLE, hdc: HANDLE) -> i32;
    fn OpenClipboard(hwnd: HANDLE) -> BOOL;
    fn EmptyClipboard() -> BOOL;
    fn SetClipboardData(format: u32, mem: HANDLE) -> HANDLE;
    fn CloseClipboard() -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    fn GetDIBits(
        hdc: HANDLE,
        hbm: HANDLE,
        start: u32,
        lines: u32,
        bits: *mut c_void,
        bmi: *mut BitmapInfo,
        usage: u32,
    ) -> i32;
    fn GetObjectW(h: HANDLE, size: i32, out: *mut c_void) -> i32;
    fn DeleteObject(h: HANDLE) -> BOOL;
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> HANDLE;
    fn GlobalLock(mem: HANDLE) -> *mut c_void;
    fn GlobalUnlock(mem: HANDLE) -> BOOL;
    fn GlobalFree(mem: HANDLE) -> HANDLE;
}

pub const SW_SHOWNORMAL: i32 = 1;

/// Öffnet eine Datei, URL oder ein Protokoll (steam://…) über Windows.
/// `verb` = None → Standardaktion, Some("runas") → als Administrator.
/// Fehler: der ShellExecute-Code (≤ 32).
pub fn shell_execute(
    verb: Option<&str>,
    file: &str,
    params: Option<&str>,
    dir: Option<&str>,
) -> Result<(), i32> {
    let verb_w = verb.map(wide);
    let file_w = wide(file);
    let params_w = params.filter(|p| !p.is_empty()).map(wide);
    let dir_w = dir.filter(|d| !d.is_empty()).map(wide);
    let r = unsafe {
        ShellExecuteW(
            null_mut(),
            verb_w.as_ref().map_or(null(), |v| v.as_ptr()),
            file_w.as_ptr(),
            params_w.as_ref().map_or(null(), |v| v.as_ptr()),
            dir_w.as_ref().map_or(null(), |v| v.as_ptr()),
            SW_SHOWNORMAL,
        )
    } as isize;
    if r > 32 {
        Ok(())
    } else {
        Err(r as i32)
    }
}

/// Holt das Icon einer EXE/DLL als RGBA-Bild (Breite, Höhe, Pixel).
pub fn extract_icon_rgba(path: &str, size: i32) -> Option<(u32, u32, Vec<u8>)> {
    let wpath = wide(path);
    let mut hicon: HANDLE = null_mut();
    let mut id: u32 = 0;
    unsafe {
        let n = PrivateExtractIconsW(wpath.as_ptr(), 0, size, size, &mut hicon, &mut id, 1, 0);
        if n == 0 || n == u32::MAX || hicon.is_null() {
            return None;
        }
        let result = icon_to_rgba(hicon);
        DestroyIcon(hicon);
        result
    }
}

unsafe fn read_bits(hbm: HANDLE, w: i32, h: i32) -> Option<Vec<u8>> {
    let mut bmi: BitmapInfo = zeroed();
    bmi.header.size = size_of::<BitmapInfoHeader>() as u32;
    bmi.header.width = w;
    bmi.header.height = -h; // oben → unten
    bmi.header.planes = 1;
    bmi.header.bit_count = 32;
    bmi.header.compression = 0; // BI_RGB
    let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
    let dc = GetDC(null_mut());
    let lines = GetDIBits(dc, hbm, 0, h as u32, buf.as_mut_ptr() as *mut c_void, &mut bmi, 0);
    ReleaseDC(null_mut(), dc);
    if lines == 0 {
        None
    } else {
        Some(buf)
    }
}

unsafe fn icon_to_rgba(hicon: HANDLE) -> Option<(u32, u32, Vec<u8>)> {
    let mut info: IconInfo = zeroed();
    if GetIconInfo(hicon, &mut info) == 0 {
        return None;
    }
    let result = (|| {
        if info.hbm_color.is_null() {
            return None;
        }
        let mut bm: Bitmap = zeroed();
        if GetObjectW(
            info.hbm_color,
            size_of::<Bitmap>() as i32,
            &mut bm as *mut Bitmap as *mut c_void,
        ) == 0
        {
            return None;
        }
        let (w, h) = (bm.bm_width, bm.bm_height);
        if w <= 0 || h <= 0 || w > 1024 || h > 1024 {
            return None;
        }
        let px = read_bits(info.hbm_color, w, h)?;
        let count = (w as usize) * (h as usize);
        let mut rgba = vec![0u8; count * 4];
        let mut has_alpha = false;
        for i in 0..count {
            rgba[i * 4] = px[i * 4 + 2];
            rgba[i * 4 + 1] = px[i * 4 + 1];
            rgba[i * 4 + 2] = px[i * 4];
            rgba[i * 4 + 3] = px[i * 4 + 3];
            if px[i * 4 + 3] != 0 {
                has_alpha = true;
            }
        }
        if !has_alpha {
            // Alte Icons ohne Alphakanal: Transparenz aus der Maske nehmen
            let mask = if info.hbm_mask.is_null() {
                None
            } else {
                read_bits(info.hbm_mask, w, h)
            };
            for i in 0..count {
                let transparent = matches!(&mask, Some(m) if m[i * 4] != 0);
                rgba[i * 4 + 3] = if transparent { 0 } else { 255 };
            }
        }
        // Komplett leeres Icon? Dann lieber keins.
        if rgba.chunks(4).all(|p| p[3] == 0) {
            return None;
        }
        Some((w as u32, h as u32, rgba))
    })();
    if !info.hbm_color.is_null() {
        DeleteObject(info.hbm_color);
    }
    if !info.hbm_mask.is_null() {
        DeleteObject(info.hbm_mask);
    }
    let _ = (info.f_icon, info.x_hotspot, info.y_hotspot);
    result
}

/// Text in die Windows-Zwischenablage kopieren.
pub fn set_clipboard(text: &str) -> bool {
    let w: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();
        let bytes = w.len() * 2;
        let mem = GlobalAlloc(0x0002, bytes); // GMEM_MOVEABLE
        if mem.is_null() {
            CloseClipboard();
            return false;
        }
        let p = GlobalLock(mem) as *mut u16;
        if p.is_null() {
            GlobalFree(mem);
            CloseClipboard();
            return false;
        }
        std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
        GlobalUnlock(mem);
        let ok = !SetClipboardData(13, mem).is_null(); // CF_UNICODETEXT
        if !ok {
            GlobalFree(mem);
        }
        CloseClipboard();
        ok
    }
}

// ----------------------------------------------------------------------
// Laufende Prozesse (für "spielt gerade")
// ----------------------------------------------------------------------

#[repr(C)]
struct ProcessEntry32W {
    size: u32,
    usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    threads: u32,
    parent_process_id: u32,
    pri_class_base: i32,
    flags: u32,
    exe_file: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> HANDLE;
    fn Process32FirstW(snap: HANDLE, entry: *mut ProcessEntry32W) -> BOOL;
    fn Process32NextW(snap: HANDLE, entry: *mut ProcessEntry32W) -> BOOL;
    fn CloseHandle(h: HANDLE) -> BOOL;
}

/// Dateinamen aller laufenden Prozesse (klein geschrieben, z. B. "witcher3.exe")
pub fn process_names() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(0x0000_0002, 0); // TH32CS_SNAPPROCESS
        if snap.is_null() || snap as isize == -1 {
            return out;
        }
        let mut e: ProcessEntry32W = zeroed();
        e.size = size_of::<ProcessEntry32W>() as u32;
        if Process32FirstW(snap, &mut e) != 0 {
            loop {
                let len = e.exe_file.iter().position(|c| *c == 0).unwrap_or(e.exe_file.len());
                out.push(String::from_utf16_lossy(&e.exe_file[..len]).to_lowercase());
                if Process32NextW(snap, &mut e) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    out
}

// ----------------------------------------------------------------------
// Zeitzone (für Uhrzeiten im Chat)
// ----------------------------------------------------------------------

#[repr(C)]
struct TimeZoneInformation {
    bias: i32,
    standard_name: [u16; 32],
    standard_date: [u16; 8],
    standard_bias: i32,
    daylight_name: [u16; 32],
    daylight_date: [u16; 8],
    daylight_bias: i32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetTimeZoneInformation(info: *mut TimeZoneInformation) -> u32;
}

/// Abstand der lokalen Zeit zu UTC in Sekunden (z. B. +3600 im Winter in Deutschland)
pub fn utc_offset_secs() -> i64 {
    unsafe {
        let mut tz: TimeZoneInformation = zeroed();
        let id = GetTimeZoneInformation(&mut tz);
        let mut bias = tz.bias;
        if id == 2 {
            bias += tz.daylight_bias;
        } else if id == 1 {
            bias += tz.standard_bias;
        }
        -(bias as i64) * 60
    }
}
