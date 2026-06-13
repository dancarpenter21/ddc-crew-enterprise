use std::{
    fs::File,
    io,
    os::fd::{AsRawFd, FromRawFd},
};

use anyhow::Context;
use nix::{
    fcntl::{OFlag, open},
    ioctl_write_ptr_bad,
    libc::{self, c_char, c_short},
    sys::stat::Mode,
};
use tokio::io::unix::AsyncFd;

const TUNSETIFF: libc::c_ulong = 0x400454ca;
const IFF_TUN: c_short = 0x0001;
const IFF_NO_PI: c_short = 0x1000;

#[repr(C)]
#[derive(Clone, Copy)]
struct IfReq {
    name: [c_char; libc::IFNAMSIZ],
    flags: c_short,
    _pad: [u8; 24],
}

ioctl_write_ptr_bad!(tun_set_iff, TUNSETIFF, IfReq);

#[derive(Debug)]
pub struct TunDevice {
    inner: AsyncFd<File>,
}

impl TunDevice {
    pub fn open(name: &str) -> anyhow::Result<Self> {
        let fd = open(
            "/dev/net/tun",
            OFlag::O_RDWR | OFlag::O_NONBLOCK,
            Mode::empty(),
        )
        .context("open /dev/net/tun")?;
        let file = unsafe { File::from_raw_fd(fd) };

        let mut ifr = IfReq {
            name: [0; libc::IFNAMSIZ],
            flags: IFF_TUN | IFF_NO_PI,
            _pad: [0; 24],
        };
        for (dst, src) in ifr.name.iter_mut().zip(name.as_bytes()) {
            *dst = *src as c_char;
        }

        unsafe {
            tun_set_iff(file.as_raw_fd(), &ifr).context("configure TUN interface")?;
        }

        Ok(Self {
            inner: AsyncFd::new(file)?,
        })
    }

    pub async fn read_packet(&self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| read_fd(inner.get_ref().as_raw_fd(), buffer)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn write_packet(&self, packet: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|inner| write_fd(inner.get_ref().as_raw_fd(), packet)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
}

fn read_fd(fd: i32, buffer: &mut [u8]) -> io::Result<usize> {
    let result = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result as usize)
    }
}

fn write_fd(fd: i32, packet: &[u8]) -> io::Result<usize> {
    let result = unsafe { libc::write(fd, packet.as_ptr().cast(), packet.len()) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result as usize)
    }
}
