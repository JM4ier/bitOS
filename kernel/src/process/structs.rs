struct Registers {
    rsp: u64,
    cr3: u64,
}

struct Process {
    registers: Registers,
}

