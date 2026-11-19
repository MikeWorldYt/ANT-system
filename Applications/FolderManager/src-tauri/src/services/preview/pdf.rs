// The first page of a PDF drawn by Windows' own PDF renderer, which needs no Adobe or other handler installed.
use std::path::Path;
use std::ffi::OsString;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::process::Command;

use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
use windows::Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};

use crate::platform::windows::powershell::CREATE_NO_WINDOW;

pub const FLAG: &str = "--pdf-thumb";
const PNG_SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G'];

pub fn is_pdf(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

fn draw(path: &Path, pixels: u32) -> windows::core::Result<Vec<u8>> {
    let bytes = std::fs::read(path).map_err(|e| windows::core::Error::new(windows::core::HRESULT(0x8007_0002u32 as i32), e.to_string()))?;
    let source = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&source)?;
    writer.WriteBytes(&bytes)?;
    writer.StoreAsync()?.join()?;
    writer.DetachStream()?;
    source.Seek(0)?;
    let document = PdfDocument::LoadFromStreamAsync(&source)?.join()?;
    let page = document.GetPage(0)?;
    let size = page.Size()?;
    let scale = pixels as f32 / size.Width.max(size.Height).max(1.0);
    let options = PdfPageRenderOptions::new()?;
    options.SetDestinationWidth(((size.Width * scale).round() as u32).max(1))?;
    options.SetDestinationHeight(((size.Height * scale).round() as u32).max(1))?;
    let out = InMemoryRandomAccessStream::new()?;
    page.RenderWithOptionsToStreamAsync(&out, &options)?.join()?;
    page.Close()?;
    let len = u32::try_from(out.Size()?).unwrap_or(u32::MAX);
    let reader = DataReader::CreateDataReader(&out.GetInputStreamAt(0)?)?;
    reader.LoadAsync(len)?.join()?;
    let mut png = vec![0; len as usize];
    reader.ReadBytes(&mut png)?;
    reader.Close()?;
    out.Close()?;
    source.Close()?;
    Ok(png)
}

// Each render runs in a child copy of the app: Windows draws PDFs on the GPU, and some drivers crash the process that used it when it exits.
pub fn page_png(path: &Path, pixels: u32) -> Option<Vec<u8>> {
    let exe = std::env::current_exe().ok()?;
    let out = Command::new(exe).arg(FLAG).arg(path).arg(pixels.to_string()).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    (out.status.success() && out.stdout.starts_with(PNG_SIGNATURE)).then_some(out.stdout)
}

// The child writes the PNG and ends at once, skipping the library teardown where the driver crashes.
pub fn run_headless(path: Option<OsString>, pixels: Option<OsString>) -> ! {
    let pixels = pixels.and_then(|p| p.to_str()?.parse().ok()).unwrap_or(128);
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    let png = if initialized { path.and_then(|p| draw(Path::new(&p), pixels).ok()) } else { None };
    let written = png.is_some_and(|png| {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&png).and_then(|()| stdout.flush()).is_ok()
    });
    unsafe { TerminateProcess(GetCurrentProcess(), u32::from(!written)) };
    std::process::exit(i32::from(!written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    // A one-page PDF built with its cross-reference offsets counted, the least Windows' renderer accepts.
    fn tiny_pdf() -> Vec<u8> {
        let objects = ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>", "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] >>"];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{body}\nendobj\n", i + 1).bytes());
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
        for o in offsets {
            out.extend(format!("{o:010} 00000 n \n").bytes());
        }
        out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).bytes());
        out
    }

    // Ignored: it draws in the test process, which the GPU driver may crash at exit; run it alone with --ignored.
    #[test]
    #[ignore]
    fn the_first_page_comes_back_as_a_png_and_a_broken_file_as_none() {
        let dir = TempDir::new("pdf-thumb");
        let good = dir.path().join("a.pdf");
        std::fs::write(&good, tiny_pdf()).unwrap();
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok().unwrap();
        let png = draw(&good, 128).expect("rendered");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

    }
}
