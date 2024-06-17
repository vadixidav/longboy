use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use longboy_proto::{Receiver, Sender, Sink, Source};

struct TestSource
{
    counter: Arc<AtomicU64>,
}

struct TestSink
{
    counter: Arc<AtomicU64>,
}

impl Source for TestSource
{
    fn poll(&mut self, buffer: &mut [u8]) -> bool
    {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        *buffer.first_chunk_mut().unwrap() = u64::to_le_bytes(counter);
        true
    }
}

impl Sink for TestSink
{
    fn handle(&mut self, input: &[u8])
    {
        let counter = u64::from_le_bytes(*input.first_chunk().unwrap());
        self.counter.fetch_max(counter, Ordering::Relaxed);
    }
}

#[test]
fn golden()
{
    let source_counter = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::new(AtomicU64::new(0));

    let mut sender = Sender::new(
        8,
        3,
        Box::new(TestSource {
            counter: source_counter.clone(),
        }),
    );
    let mut receiver = Receiver::new(
        8,
        3,
        Box::new(TestSink {
            counter: sink_counter.clone(),
        }),
    );

    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 0);

    // 1
    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 1);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 1);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 0);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 1);
    assert_eq!(receiver.cycle(), 1);
    assert_eq!(source_counter.load(Ordering::Relaxed), 1);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 1);

    // 2
    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 2);
    assert_eq!(receiver.cycle(), 1);
    assert_eq!(source_counter.load(Ordering::Relaxed), 2);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 1);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 2);
    assert_eq!(receiver.cycle(), 2);
    assert_eq!(source_counter.load(Ordering::Relaxed), 2);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 2);

    // 3
    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 2);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 2);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 3);

    // 4
    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 4);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 4);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 3);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 4);
    assert_eq!(receiver.cycle(), 4);
    assert_eq!(source_counter.load(Ordering::Relaxed), 4);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 4);

    // 5
    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 5);
    assert_eq!(receiver.cycle(), 4);
    assert_eq!(source_counter.load(Ordering::Relaxed), 5);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 4);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 5);
    assert_eq!(receiver.cycle(), 5);
    assert_eq!(source_counter.load(Ordering::Relaxed), 5);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 5);

    // 6
    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 6);
    assert_eq!(receiver.cycle(), 5);
    assert_eq!(source_counter.load(Ordering::Relaxed), 6);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 5);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 6);
    assert_eq!(receiver.cycle(), 6);
    assert_eq!(source_counter.load(Ordering::Relaxed), 6);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 6);
}

#[test]
fn mirroring()
{
    let source_counter = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::new(AtomicU64::new(0));

    let mut sender = Sender::new(
        8,
        3,
        Box::new(TestSource {
            counter: source_counter.clone(),
        }),
    );
    let mut receiver = Receiver::new(
        8,
        3,
        Box::new(TestSink {
            counter: sink_counter.clone(),
        }),
    );

    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 0);

    let datagram_1: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_2: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_3: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 0);

    receiver.handle_datagram(&datagram_1);
    receiver.handle_datagram(&datagram_1);
    receiver.handle_datagram(&datagram_1);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 1);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 1);

    receiver.handle_datagram(&datagram_2);
    receiver.handle_datagram(&datagram_2);
    receiver.handle_datagram(&datagram_2);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 2);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 2);

    receiver.handle_datagram(&datagram_3);
    receiver.handle_datagram(&datagram_3);
    receiver.handle_datagram(&datagram_3);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 3);
}

#[test]
fn out_of_order()
{
    let source_counter = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::new(AtomicU64::new(0));

    let mut sender = Sender::new(
        8,
        3,
        Box::new(TestSource {
            counter: source_counter.clone(),
        }),
    );
    let mut receiver = Receiver::new(
        8,
        3,
        Box::new(TestSink {
            counter: sink_counter.clone(),
        }),
    );

    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 0);

    // Inside of window.
    let datagram_1: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_2: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_3: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 0);

    receiver.handle_datagram(&datagram_3);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 3);

    receiver.handle_datagram(&datagram_2);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 3);

    receiver.handle_datagram(&datagram_1);
    assert_eq!(sender.cycle(), 3);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 3);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 3);

    // Outside of window.
    let datagram_4: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_5: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_6: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_7: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_8: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_9: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_10: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    let datagram_11: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());

    receiver.handle_datagram(&datagram_11);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_10);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_9);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_8);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_7);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 3);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_6);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 11);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_5);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 11);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);

    receiver.handle_datagram(&datagram_4);
    assert_eq!(sender.cycle(), 11);
    assert_eq!(receiver.cycle(), 11);
    assert_eq!(source_counter.load(Ordering::Relaxed), 11);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 11);
}

#[test]
fn lost_in_transmission()
{
    let source_counter = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::new(AtomicU64::new(0));

    let mut sender = Sender::new(
        8,
        3,
        Box::new(TestSource {
            counter: source_counter.clone(),
        }),
    );
    let mut receiver = Receiver::new(
        8,
        3,
        Box::new(TestSink {
            counter: sink_counter.clone(),
        }),
    );

    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 0);

    for _ in 0..128
    {
        sender.poll_datagram().unwrap();
    }
    assert_eq!(sender.cycle(), 128);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 128);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 0);

    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 129);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 129);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 0);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 129);
    assert_eq!(receiver.cycle(), 121);
    assert_eq!(source_counter.load(Ordering::Relaxed), 129);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 129);
}

#[test]
fn cycle_wrapping()
{
    let source_counter = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::new(AtomicU64::new(0));

    let mut sender = Sender::new(
        8,
        3,
        Box::new(TestSource {
            counter: source_counter.clone(),
        }),
    );
    let mut receiver = Receiver::new(
        8,
        3,
        Box::new(TestSink {
            counter: sink_counter.clone(),
        }),
    );

    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 0);

    for _ in 0..(65535 - 1)
    {
        let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
        receiver.handle_datagram(&datagram);
        println!("{}", receiver.cycle());
    }
    assert_eq!(sender.cycle(), 65534);
    assert_eq!(receiver.cycle(), 65534);
    assert_eq!(source_counter.load(Ordering::Relaxed), 65534);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 65534);

    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 65534);
    assert_eq!(source_counter.load(Ordering::Relaxed), 65535);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 65534);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 0);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 65535);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 65535);

    let datagram: Box<[u8]> = Box::from(sender.poll_datagram().unwrap());
    assert_eq!(sender.cycle(), 1);
    assert_eq!(receiver.cycle(), 0);
    assert_eq!(source_counter.load(Ordering::Relaxed), 65536);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 65535);

    receiver.handle_datagram(&datagram);
    assert_eq!(sender.cycle(), 1);
    assert_eq!(receiver.cycle(), 1);
    assert_eq!(source_counter.load(Ordering::Relaxed), 65536);
    assert_eq!(sink_counter.load(Ordering::Relaxed), 65536);
}
