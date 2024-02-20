use alloc::sync::Arc;
use axhal::mem::PhysAddr;
use futures_intrusive::sync::{GenericMutex, GenericMutexGuard};
use futures_util::task::AtomicWaker;
use spinlock::{BaseSpinLock, BaseSpinLockGuard, SpinNoIrq};
use spinning_top::{RawSpinlock, Spinlock};
use xhci::ring::trb::{
    command::{self, AddressDevice, ConfigureEndpoint, EnableSlot, EvaluateContext},
    event::{self, CompletionCode},
};

use crate::host::xhci::{
    command_ring::{command_type::CommandType, CommandRing},
    event_ring::EventRing,
};

use super::receiver::{self, ReceiveFuture};

static mut SENDER: Option<Arc<GenericMutex<RawSpinlock, Sender>>> = None;

pub(crate) fn init(r: Arc<Spinlock<CommandRing>>) {
    // SENDER
    //     .try_init_once(|| Futurelock::new(Sender::new(r), true))
    //     .expect("`Sender` is initialized more than once.");
    unsafe {
        if !SENDER.is_none() {
            SENDER = Some(Arc::new(GenericMutex::new(Sender::new(r), true)))
        }
    }
}

pub(crate) async fn enable_device_slot() -> u8 {
    lock().await.enable_device_slot().await
}

pub(crate) async fn address_device(input_cx: PhysAddr, slot: u8) {
    lock().await.address_device(input_cx, slot).await;
}

pub(crate) async fn configure_endpoint(cx: PhysAddr, slot: u8) {
    lock().await.configure_endpoint(cx, slot).await;
}

pub(crate) async fn evaluate_context(cx: PhysAddr, slot: u8) {
    lock().await.evaluate_context(cx, slot).await;
}

async fn lock() -> GenericMutexGuard<'static, RawSpinlock, Sender> {
    let s = unsafe { SENDER.as_ref().expect("`SENDER` is not initialized.") };
    s.lock().await
}

struct Sender {
    channel: Channel,
}
impl Sender {
    fn new(ring: Arc<Spinlock<CommandRing>>) -> Sender {
        Self {
            channel: unsafe { Channel::new(ring) },
        }
    }

    async fn enable_device_slot(&mut self) -> u8 {
        let t = EnableSlot::default();
        let completion = self.send_and_receive(t.into()).await;
        panic_on_error("Enable Device Slot", completion);
        if let event::Allowed::CommandCompletion(c) = completion {
            c.slot_id()
        } else {
            unreachable!()
        }
    }

    async fn address_device(&mut self, input_context_addr: PhysAddr, slot_id: u8) {
        let t = *AddressDevice::default()
            .set_input_context_pointer(input_context_addr.as_usize() as u64)
            .set_slot_id(slot_id);
        let c = self.send_and_receive(t.into()).await;
        panic_on_error("Address Device", c);
    }

    async fn configure_endpoint(&mut self, context_addr: PhysAddr, slot_id: u8) {
        let t = *ConfigureEndpoint::default()
            .set_input_context_pointer(context_addr.as_usize() as u64)
            .set_slot_id(slot_id);
        let c = self.send_and_receive(t.into()).await;
        panic_on_error("Configure Endpoint", c);
    }

    async fn evaluate_context(&mut self, cx: PhysAddr, slot: u8) {
        let t = *EvaluateContext::default()
            .set_input_context_pointer(cx.as_usize() as u64)
            .set_slot_id(slot);
        let c = self.send_and_receive(t.into()).await;
        panic_on_error("Evaluate Context", c);
    }

    async fn send_and_receive(&mut self, t: command::Allowed) -> event::Allowed {
        self.channel.send_and_receive(t).await
    }
}

struct Channel {
    ring: Arc<Spinlock<CommandRing>>,
    waker: Arc<Spinlock<AtomicWaker>>,
}
impl Channel {
    unsafe fn new(ring: Arc<Spinlock<CommandRing>>) -> Channel {
        Self {
            ring,
            waker: Arc::new(Spinlock::new(AtomicWaker::new())),
        }
    }

    async fn send_and_receive(&mut self, t: command::Allowed) -> event::Allowed {
        let a = self.ring.lock().enqueue(t);
        self.register_with_receiver(a);
        self.get_trb(a).await
    }

    fn register_with_receiver(&mut self, trb_a: PhysAddr) {
        receiver::add_entry(trb_a, self.waker.clone()).expect("Sender is already registered.");
    }

    async fn get_trb(&mut self, trb_a: PhysAddr) -> event::Allowed {
        ReceiveFuture::new(trb_a, self.waker.clone()).await
    }
}

fn panic_on_error(n: &str, c: event::Allowed) {
    if let event::Allowed::CommandCompletion(c) = c {
        assert!(
            c.completion_code() == Ok(CompletionCode::Success),
            "{} command failed: {:?}",
            n,
            c.completion_code()
        );
    } else {
        unreachable!("The Command Completion TRB is the only TRB to receive in response to the Command TRBs.");
    }
}