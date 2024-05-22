use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use log::debug;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::{
    context::{Device, Device64Byte, DeviceHandler},
    ring::trb::event::{CompletionCode, TransferEvent},
};

use super::{event_ring::TypeXhciTrb, registers};

const XHCI_CONFIG_MAX_SLOTS: usize = 64;
pub(crate) struct SlotManager {
    dcbaa: PageBox<[VirtAddr]>,
    // device: PageBox<[Device64Byte]>,
}

impl SlotManager {
    pub fn assign_device(&mut self, valid_slot_id: u8, device: VirtAddr) {
        debug!("assign device: {:?} to dcbaa {}", device, valid_slot_id);

        // self.device[valid_slot_id as usize - 1] = device;
        self.dcbaa[valid_slot_id as usize] = device
        //TODO 需要考虑内存同步问题
        //TODO 内存位置可能不对
    }

    pub fn deref_device_at(&self, slot_id: usize) -> Device64Byte {
        unsafe { *(self.dcbaa[slot_id].as_mut_ptr() as *mut Device64Byte) }
    }
}

pub(crate) static SLOT_MANAGER: OnceCell<Spinlock<SlotManager>> = OnceCell::uninit();

pub(crate) fn transfer_event(
    uch_completion_code: CompletionCode,
    trb: TransferEvent,
) -> Result<TypeXhciTrb, ()> {
    assert!((1 <= trb.slot_id()) && (usize::from(trb.slot_id()) <= XHCI_CONFIG_MAX_SLOTS));
    // TODO: check device exists
    // let slot_manager = SLOT_MANAGER.try_get().unwrap().lock();
    // // let device = &slot_manager.device[(uch_slot_id - 1) as usize] as &Device64Byte;
    // let endpoint = slot_manager
    //     .deref_device_at(uch_slot_id as usize)
    //     .endpoint((uch_endpoint_id - 1).try_into().unwrap());
    debug!("transfer event! param: {:?},{:?}", uch_completion_code, trb);
    match uch_completion_code {
        CompletionCode::Success => {
            debug!("transfer event succeed!");
            Ok(trb.into_raw())
        }
        any => {
            debug!("failed, code:{:?}", any);
            Err(())
        }
    }
    // TODO: event transfer
}

pub(crate) fn new() {
    registers::handle(|r| {
        let slot_manager = SlotManager {
            dcbaa: PageBox::new_slice(VirtAddr::from(0 as usize), XHCI_CONFIG_MAX_SLOTS + 1),
            // device: PageBox::new_slice(Device::new_64byte(), XHCI_CONFIG_MAX_SLOTS + 1),
        };

        r.operational
            .dcbaap
            .update_volatile(|d| d.set(slot_manager.dcbaa.virt_addr().as_usize() as u64));

        let max_device_slots_enabled = r
            .operational
            .config
            .read_volatile()
            .max_device_slots_enabled();

        debug!("max slot: {}", max_device_slots_enabled); // return 0, not good!

        r.operational.config.update_volatile(|cfg| {
            // cfg.set_max_device_slots_enabled(max_device_slots_enabled);
            // cfg.set_max_device_slots_enabled(2); // lets just hard code: 2
            cfg.set_max_device_slots_enabled(128); // lets just hard code: 128
        });

        SLOT_MANAGER
            .try_init_once(move || Spinlock::new(slot_manager))
            .expect("Failed to initialize `SlotManager`.");
    });
    debug!("initialized!");
}

pub fn set_dcbaa(buffer_array: &[VirtAddr]) {
    let mut dcbaa_box = &mut SLOT_MANAGER.get().unwrap().lock().dcbaa;
    buffer_array
        .iter()
        .zip(dcbaa_box.iter_mut())
        .for_each(|(l, r)| *r = *l);
}
