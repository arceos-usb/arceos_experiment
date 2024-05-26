use super::registers;
use crate::mapper::Mapper;
use conquer_once::spin::OnceCell;
use core::convert::TryInto;
use spinning_top::Spinlock;

use xhci::{extended_capabilities, ExtendedCapability};

static EXTENDED_CAPABILITIES: OnceCell<Spinlock<Option<extended_capabilities::List<Mapper>>>> =
    OnceCell::uninit();

/// # Safety
///
/// `mmio_base` must be the correct one.
pub(crate) unsafe fn init(mmio_base: VirtAddr) {
    let hccparams1 = registers::handle(|r| r.capability.hccparams1.read_volatile());

    EXTENDED_CAPABILITIES
        .try_init_once(|| {
            Spinlock::new(extended_capabilities::List::new(
                mmio_base.as_u64().try_into().unwrap(),
                hccparams1,
                Mapper,
            ))
        })
        .expect("Failed to initialize `EXTENDED_CAPABILITIES`.");
}

pub(crate) fn iter() -> Option<
    impl Iterator<Item = Result<ExtendedCapability<Mapper>, extended_capabilities::NotSupportedId>>,
> {
    Some(
        EXTENDED_CAPABILITIES
            .try_get()
            .expect("`EXTENDED_CAPABILITIES` is not initialized.`")
            .lock()
            .as_mut()?
            .into_iter(),
    )
}
