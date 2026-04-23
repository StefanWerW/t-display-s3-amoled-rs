#![no_std]
#![no_main]

extern crate alloc;
use core::mem::MaybeUninit;

use esp_backtrace as _;
use esp_println::println;
use hal::{
    delay::Delay,
    gpio::{Output, Input, Level, Pull, OutputConfig, InputConfig},
};

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[unsafe(export_name = "esp_app_desc")]
#[unsafe(link_section = ".rodata_desc")]
#[used]
pub static ESP_APP_DESC: esp_bootloader_esp_idf::EspAppDesc = esp_bootloader_esp_idf::EspAppDesc::new_internal(
    env!("CARGO_PKG_VERSION"),
    env!("CARGO_PKG_NAME"),
    "00:00:00",
    "2026-04-23",
    "esp-hal",
    0,
    u16::MAX,
    65536,
    0,
);

fn init_heap() {    
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[hal::main]
fn main() -> ! {
    init_heap();
    let peripherals = hal::init(hal::Config::default());

    println!("Hello world!");

    let mut led = Output::new(peripherals.GPIO38, Level::High, OutputConfig::default());
    let _button = Input::new(peripherals.GPIO21, InputConfig::default().with_pull(Pull::Down));

    let delay = Delay::new();

    loop {
        led.toggle();
        delay.delay_millis(1000);
    }
}
