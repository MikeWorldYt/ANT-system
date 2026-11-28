// Cluster size of the volume that holds a path, which a file's size on disk is rounded up to.
use std::path::Path;

const DEFAULT_CLUSTER: u64 = 4096;

#[cfg(windows)]
pub fn cluster_size(path: &Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceW, GetVolumePathNameW};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut root = [0u16; 261];
    let (mut sectors, mut bytes, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    // SAFETY: every pointer is valid for the call and the sizes match the buffers they describe.
    let ok = unsafe { GetVolumePathNameW(wide.as_ptr(), root.as_mut_ptr(), root.len() as u32) != 0 && GetDiskFreeSpaceW(root.as_ptr(), &mut sectors, &mut bytes, &mut free, &mut total) != 0 };
    let size = u64::from(sectors) * u64::from(bytes);
    if ok && size > 0 { size } else { DEFAULT_CLUSTER }
}

#[cfg(not(windows))]
pub fn cluster_size(_path: &Path) -> u64 {
    DEFAULT_CLUSTER
}
