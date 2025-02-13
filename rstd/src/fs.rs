pub fn open(str: &str, mode: usize) -> isize {
    syscall!(sc::nr::OPEN, str.as_ptr() as usize, mode, str.len())
}

pub fn close(fd: usize) {
    syscall!(sc::nr::CLOSE, fd);
}

pub fn read(fd: usize, buf: usize, len: usize) -> isize {
    syscall!(sc::nr::READ, fd, buf, len)
}

pub fn write(fd: usize, buf: usize, len: usize) -> isize {
    syscall!(sc::nr::WRITE, fd, buf, len)
}

pub fn fstat(fd: usize, buf: usize) -> isize {
    syscall!(sc::nr::FSTAT, fd, buf)
}

pub fn pipe(fd: usize) -> isize {
    syscall!(sc::nr::PIPE, fd)
}

pub fn registfs(fs_name: &str, fs_addr: usize) -> isize {
    syscall!(10003, fs_name.as_ptr() as usize, fs_name.len(), fs_addr)
}
