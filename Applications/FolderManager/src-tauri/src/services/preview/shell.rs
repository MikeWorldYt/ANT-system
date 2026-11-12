// Windows file images come from the shell, so icons and thumbnails match Explorer for every file type.
use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;

use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF, SIIGBF_ICONONLY};

use super::png_data_url;
use crate::domain::error::{FmError, R};

const THUMBNAIL_PIXELS: i32 = 128;
const ICON_PIXELS: i32 = 16;
// Enough for the side panel width, far below a photo's own size.
const PREVIEW_PIXELS: i32 = 320;

// The grid gets the file's thumbnail, falling back to its type icon; list rows get the small icon only.
pub fn image(path: &Path, large: bool) -> R<Option<String>> {
    let (pixels, flags) = if large { (THUMBNAIL_PIXELS, SIIGBF(0)) } else { (ICON_PIXELS, SIIGBF_ICONONLY) };
    sized(path, pixels, flags)
}

pub fn preview(path: &Path) -> R<Option<String>> {
    sized(path, PREVIEW_PIXELS, SIIGBF(0))
}

fn sized(path: &Path, pixels: i32, flags: SIIGBF) -> R<Option<String>> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = bitmap_url(path, pixels, flags);
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

fn bitmap_url(path: &Path, pixels: i32, flags: SIIGBF) -> R<Option<String>> {
    let name = HSTRING::from(path.to_string_lossy().as_ref());
    let factory: IShellItemImageFactory = unsafe { SHCreateItemFromParsingName(&name, None) }
        .map_err(|e| FmError::Internal { message: format!("Windows could not open {}: {e}", path.display()) })?;
    let Ok(bitmap) = (unsafe { factory.GetImage(SIZE { cx: pixels, cy: pixels }, flags) }) else {
        return Ok(None);
    };
    let pixels = read_rgba(bitmap);
    let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
    Ok(pixels.map(|(width, height, rgba)| png_data_url(width, height, &rgba)))
}

// Reads the bitmap as top-down BGRA and turns it into RGBA; shell icons without alpha are made opaque.
fn read_rgba(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut info_bitmap = BITMAP::default();
    let copied = unsafe { GetObjectW(HGDIOBJ(bitmap.0), size_of::<BITMAP>() as i32, Some(&mut info_bitmap as *mut BITMAP as *mut c_void)) };
    if copied == 0 || info_bitmap.bmWidth <= 0 || info_bitmap.bmHeight <= 0 {
        return None;
    }
    let (width, height) = (info_bitmap.bmWidth, info_bitmap.bmHeight);
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bgra = vec![0u8; width as usize * height as usize * 4];
    let hdc = unsafe { CreateCompatibleDC(None) };
    let lines = unsafe { GetDIBits(hdc, bitmap, 0, height as u32, Some(bgra.as_mut_ptr().cast()), &mut header, DIB_RGB_COLORS) };
    let _ = unsafe { DeleteDC(hdc) };
    if lines == 0 {
        return None;
    }
    let opaque = bgra.chunks(4).all(|px| px[3] == 0);
    let mut rgba = Vec::with_capacity(bgra.len());
    for px in bgra.chunks(4) {
        rgba.extend_from_slice(&[px[2], px[1], px[0], if opaque { 255 } else { px[3] }]);
    }
    Some((width as u32, height as u32, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_file_has_an_icon_and_a_thumbnail() {
        let dir = crate::test_support::TempDir::new("shell-image");
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, "hello").unwrap();
        for large in [false, true] {
            let url = image(&file, large).unwrap().expect("the shell draws a type icon");
            assert!(url.starts_with("data:image/png;base64,"), "{url}");
        }
    }
}
