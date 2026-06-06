//! Single-threaded shared I2C bus adapter.
//!
//! The Waveshare sample places the PMIC, RTC, environmental sensor and IMU on
//! one I2C bus. The first Rust milestones gave the bus exclusively to the PMIC
//! because only the panel rail was required. This adapter keeps ownership
//! explicit while allowing small protocol drivers to share that verified bus.

use std::{cell::RefCell, rc::Rc};

use embedded_hal::i2c::{ErrorType, I2c, Operation};

/// Cloneable single-threaded owner for one blocking I2C bus.
///
/// The firmware event loop is synchronous, so transactions never overlap.
/// `RefCell` protects against accidental nested mutable access during future
/// extensions without introducing an async executor or RTOS mutex.
pub struct SharedI2cBus<I2C> {
    inner: Rc<RefCell<I2C>>,
}

impl<I2C> SharedI2cBus<I2C> {
    /// Wrap one HAL I2C driver for sharing between board-service modules.
    #[must_use]
    pub fn new(i2c: I2C) -> Self {
        Self {
            inner: Rc::new(RefCell::new(i2c)),
        }
    }
}

impl<I2C> Clone for SharedI2cBus<I2C> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<I2C> ErrorType for SharedI2cBus<I2C>
where
    I2C: ErrorType,
{
    type Error = I2C::Error;
}

impl<I2C> I2c for SharedI2cBus<I2C>
where
    I2C: I2c,
{
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.inner.borrow_mut().transaction(address, operations)
    }
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;
    use std::{cell::Cell, rc::Rc};

    use embedded_hal::i2c::{ErrorType, I2c, Operation};

    use super::SharedI2cBus;

    struct CountingI2c {
        transaction_count: Rc<Cell<u32>>,
    }

    impl ErrorType for CountingI2c {
        type Error = Infallible;
    }

    impl I2c for CountingI2c {
        fn transaction(
            &mut self,
            _address: u8,
            _operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            self.transaction_count
                .set(self.transaction_count.get().saturating_add(1));
            Ok(())
        }
    }

    #[test]
    fn clones_forward_transactions_to_one_bus() {
        let count = Rc::new(Cell::new(0));
        let mut first = SharedI2cBus::new(CountingI2c {
            transaction_count: Rc::clone(&count),
        });
        let mut second = first.clone();

        first.write(0x34, &[0x03]).unwrap();
        second.write(0x51, &[0x04]).unwrap();

        assert_eq!(count.get(), 2);
    }
}
