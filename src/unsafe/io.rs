use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::slice;

pub struct RawIO {
    fd: RawFd,
    owned: bool,
}

impl RawIO {
    pub unsafe fn from_raw_fd(fd: RawFd, owned: bool) -> Self {
        Self { fd, owned }
    }

    pub unsafe fn from_file(file: File) -> Self {
        let fd = file.as_raw_fd();
        mem::forget(file);
        Self { fd, owned: true }
    }

    pub fn raw_fd(&self) -> RawFd {
        self.fd
    }

    pub unsafe fn read_direct(&self, buf: *mut u8, len: usize) -> io::Result<usize> {
        let ret = libc::read(self.fd, buf as *mut libc::c_void, len);
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    pub unsafe fn write_direct(&self, buf: *const u8, len: usize) -> io::Result<usize> {
        let ret = libc::write(self.fd, buf as *const libc::c_void, len);
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    pub fn seek(&self, pos: i64, whence: i32) -> io::Result<u64> {
        let ret = unsafe { libc::lseek(self.fd, pos, whence) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as u64)
        }
    }

    pub fn fsync(&self) -> io::Result<()> {
        let ret = unsafe { libc::fsync(self.fd) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub unsafe fn mmap(
        &self,
        len: usize,
        prot: i32,
        flags: i32,
        offset: i64,
    ) -> io::Result<*mut u8> {
        let ptr = libc::mmap(ptr::null_mut(), len, prot, flags, self.fd, offset);

        if ptr == libc::MAP_FAILED {
            Err(io::Error::last_os_error())
        } else {
            Ok(ptr as *mut u8)
        }
    }

    pub unsafe fn munmap(&self, addr: *mut u8, len: usize) -> io::Result<()> {
        let ret = libc::munmap(addr as *mut libc::c_void, len);
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub unsafe fn madvise(&self, addr: *mut u8, len: usize, advice: i32) -> io::Result<()> {
        let ret = libc::madvise(addr as *mut libc::c_void, len, advice);
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn pread(&self, buf: &mut [u8], offset: i64) -> io::Result<usize> {
        let ret = unsafe {
            libc::pread(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                offset,
            )
        };

        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    pub fn pwrite(&self, buf: &[u8], offset: i64) -> io::Result<usize> {
        let ret = unsafe {
            libc::pwrite(
                self.fd,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
                offset,
            )
        };

        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    pub fn readv(&self, iovecs: &mut [libc::iovec]) -> io::Result<usize> {
        let ret = unsafe { libc::readv(self.fd, iovecs.as_mut_ptr(), iovecs.len() as i32) };

        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    pub fn writev(&self, iovecs: &[libc::iovec]) -> io::Result<usize> {
        let ret = unsafe { libc::writev(self.fd, iovecs.as_ptr(), iovecs.len() as i32) };

        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    pub unsafe fn read_vectored_direct(&self, bufs: &mut [&mut [u8]]) -> io::Result<usize> {
        let mut iovecs: Vec<libc::iovec> = Vec::with_capacity(bufs.len());

        for buf in bufs.iter_mut() {
            iovecs.push(libc::iovec {
                iov_base: buf.as_mut_ptr() as *mut libc::c_void,
                iov_len: buf.len(),
            });
        }

        self.readv(&mut iovecs)
    }

    pub unsafe fn write_vectored_direct(&self, bufs: &[&[u8]]) -> io::Result<usize> {
        let mut iovecs: Vec<libc::iovec> = Vec::with_capacity(bufs.len());

        for buf in bufs.iter() {
            iovecs.push(libc::iovec {
                iov_base: buf.as_ptr() as *mut libc::c_void,
                iov_len: buf.len(),
            });
        }

        self.writev(&iovecs)
    }

    pub fn allocate(&self, offset: i64, len: i64) -> io::Result<()> {
        let ret = unsafe { libc::fallocate(self.fd, 0, offset, len) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn truncate(&self, len: i64) -> io::Result<()> {
        let ret = unsafe { libc::ftruncate(self.fd, len) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for RawIO {
    fn drop(&mut self) {
        if self.owned {
            unsafe { libc::close(self.fd) };
        }
    }
}

pub struct MemoryMappedFile {
    addr: *mut u8,
    len: usize,
    io: Option<RawIO>,
}

impl MemoryMappedFile {
    pub unsafe fn new(file: File, len: usize, write: bool) -> io::Result<Self> {
        let io = RawIO::from_file(file);

        let prot = libc::PROT_READ | if write { libc::PROT_WRITE } else { 0 };
        let flags = libc::MAP_SHARED;

        let addr = io.mmap(len, prot, flags, 0)?;

        Ok(Self {
            addr,
            len,
            io: Some(io),
        })
    }

    pub unsafe fn anonymous(len: usize) -> io::Result<Self> {
        let prot = libc::PROT_READ | libc::PROT_WRITE;
        let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;

        let addr = libc::mmap(ptr::null_mut(), len, prot, flags, -1, 0);

        if addr == libc::MAP_FAILED {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self {
                addr: addr as *mut u8,
                len,
                io: None,
            })
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.addr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.addr, self.len) }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.addr
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.addr
    }

    pub fn advise(&self, advice: i32) -> io::Result<()> {
        if let Some(ref io) = self.io {
            unsafe { io.madvise(self.addr, self.len, advice) }
        } else {
            unsafe {
                let ret = libc::madvise(self.addr as *mut libc::c_void, self.len, advice);
                if ret < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn sync(&self, sync_flags: i32) -> io::Result<()> {
        unsafe {
            let ret = libc::msync(self.addr as *mut libc::c_void, self.len, sync_flags);
            if ret < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for MemoryMappedFile {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.addr as *mut libc::c_void, self.len);
        }
    }
}

pub fn direct_copy(src: &RawIO, dst: &RawIO, buffer_size: usize) -> io::Result<u64> {
    let mut buffer = Vec::with_capacity(buffer_size);
    unsafe {
        buffer.set_len(buffer_size);
    }

    let mut total_copied = 0;

    loop {
        let read_bytes = unsafe { src.read_direct(buffer.as_mut_ptr(), buffer_size) }?;
        if read_bytes == 0 {
            break;
        }

        let mut written = 0;
        while written < read_bytes {
            let n =
                unsafe { dst.write_direct(buffer.as_ptr().add(written), read_bytes - written) }?;

            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "failed to write"));
            }

            written += n;
        }

        total_copied += read_bytes as u64;
    }

    Ok(total_copied)
}

#[cfg(target_os = "linux")]
pub fn splice_copy(src: &RawIO, dst: &RawIO, len: usize) -> io::Result<u64> {
    let mut total = 0;
    let mut remaining = len;

    while remaining > 0 {
        let chunk_size = remaining.min(0x7ffff000);
        let ret = unsafe {
            libc::splice(
                src.raw_fd(),
                ptr::null_mut(),
                dst.raw_fd(),
                ptr::null_mut(),
                chunk_size,
                libc::SPLICE_F_MOVE,
            )
        };

        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        if ret == 0 {
            break;
        }

        total += ret as u64;
        remaining -= ret as usize;
    }

    Ok(total)
}
