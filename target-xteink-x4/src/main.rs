#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(target_arch = "riscv32")]
mod rustmix_x4;

#[cfg(not(target_arch = "riscv32"))]
fn main() {
    println!("Rustmix X4 host placeholder: rustmix=x4-runtime-ready");
}
