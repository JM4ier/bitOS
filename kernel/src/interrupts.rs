use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, HandlerFunc};
use crate::{print, println};
use lazy_static::lazy_static;
use crate::gdt;
use pic8259_simple::ChainedPics;
use spin;
use crate::hlt_loop;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);
        idt.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        unsafe{idt.page_fault.set_handler_fn(page_fault_handler).set_stack_index(gdt::PAGE_FAULT_IST_INDEX);}
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);
        idt.security_exception.set_handler_fn(security_exception_handler);
        unsafe {
            idt[0x80].set_handler_fn(
                *(&(syscall_handler as unsafe extern "C" fn())
                    as *const unsafe extern "C" fn()
                    as u64 as *const HandlerFunc)
            );
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "C" {
    pub fn syscall_handler();
}
global_asm!(include_str!("syscall_handler.s"));

extern "x86-interrupt" fn divide_by_zero_handler (stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: DIVIDED BY ZERO\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn debug_handler (stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn non_maskable_interrupt_handler (stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: NON MASKABLE INTERRUPT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_tss_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: INVALID TSS\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: SEGMENT NOT PRESENT\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn stack_segment_fault_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: STACK SEGMENT FAULT\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn x87_floating_point_handler (stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: X87 FLOATING POINT\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn alignment_check_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: ALIGNMENT CHECK\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn security_exception_handler (stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: SECURITY EXCEPTION\n{:#?}", stack_frame);
    println!("ERROR CODE: {}", error_code);
    hlt_loop();
}

extern "x86-interrupt" fn machine_check_handler (stack_frame: &mut InterruptStackFrame) {
    println!("MACHINE CHECK\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn simd_floating_point_handler (stack_frame: &mut InterruptStackFrame) {
    println!("SIMD FLOATING POINT\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn virtualization_handler (stack_frame: &mut InterruptStackFrame) {
    println!("VIRTUALIZATION\n{:#?}", stack_frame);
    hlt_loop();
}

// interrupt handling
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
spin::Mutex::new(unsafe{ ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    // print!(".");
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    use pc_keyboard::{Keyboard, ScancodeSet1, DecodedKey, layouts};
    use spin::Mutex;
    use crate::vga_buffer;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if scancode == 14 {
            // backspace
            vga_buffer::backspace();
        } else if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

use x86_64::structures::idt::PageFaultErrorCode;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Acessed Address: {:?}", Cr2::read());
    println!("Error code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

