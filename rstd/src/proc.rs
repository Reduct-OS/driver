pub fn exit(code: usize) -> isize {
    syscall!(sc::nr::EXIT, code)
}
