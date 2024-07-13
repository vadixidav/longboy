use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::Arc,
};

use enum_map::enum_map;
use flume::{Receiver as FlumeReceiver, Sender as FlumeSender};
use parking_lot::Mutex;

use longboy::{
    Client, ClientToServerSchema, Factory, Mirroring, Runtime, RuntimeTask, Server, ServerToClientSchema, Session,
};
use longboy_proto::{Sink, Source};

#[derive(Clone)]
struct TestSession
{
    session_id: u64,
    cipher_key: u64,
}

#[derive(Clone)]
struct TestRuntime
{
    inner: Arc<Mutex<TestRuntimeInner>>,
}

struct TestRuntimeInner
{
    tick_period: u16,

    timestamp: u16,
    tasks: Vec<Box<dyn RuntimeTask>>,
}

struct TestServerToClientSourceFactory
{
    channels: [FlumeReceiver<(u32, [u64; 2])>; 2],
}

struct TestClientToServerSinkFactory
{
    channel: FlumeSender<(u32, u8, u64)>,
}

struct TestClientToServerSource
{
    channel: FlumeReceiver<(u32, u64)>,
}

struct TestClientToServerSink
{
    player_index: u8,
    channel: FlumeSender<(u32, u8, u64)>,
}

struct TestServerToClientSource
{
    channel: FlumeReceiver<(u32, [u64; 2])>,
}

struct TestServerToClientSink
{
    channel: FlumeSender<(u32, [u64; 2])>,
}

impl Session for TestSession
{
    fn ip_addr(&self) -> IpAddr
    {
        IpAddr::V4(Ipv4Addr::from([127, 0, 0, 1]))
    }

    fn session_id(&self) -> u64
    {
        self.session_id
    }

    fn cipher_key(&self) -> u64
    {
        self.cipher_key
    }
}

impl TestRuntime
{
    fn new(tick_period: u16) -> Self
    {
        Self {
            inner: Arc::new(Mutex::new(TestRuntimeInner {
                tick_period: tick_period,

                timestamp: 0,
                tasks: Vec::new(),
            })),
        }
    }

    fn tick(&self)
    {
        let mut inner = self.inner.lock();
        inner.timestamp += inner.tick_period;

        let timestamp = inner.timestamp;
        inner.tasks.iter_mut().for_each(|task| task.poll(timestamp));
    }
}

impl Runtime for TestRuntime
{
    fn running(&self) -> bool
    {
        true
    }

    fn spawn(&mut self, task: Box<dyn longboy::RuntimeTask>)
    {
        self.inner.lock().tasks.push(task);
    }
}

impl Factory for TestServerToClientSourceFactory
{
    type Type = TestServerToClientSource;

    fn invoke(&mut self, session_id: u64) -> Self::Type
    {
        let player_index = match session_id
        {
            1 => 0,
            2 => 1,
            _ => unreachable!(),
        };

        TestServerToClientSource {
            channel: self.channels[player_index].clone(),
        }
    }
}

impl Factory for TestClientToServerSinkFactory
{
    type Type = TestClientToServerSink;

    fn invoke(&mut self, session_id: u64) -> Self::Type
    {
        let player_index = match session_id
        {
            1 => 0,
            2 => 1,
            _ => unreachable!(),
        };

        TestClientToServerSink {
            player_index: player_index,
            channel: self.channel.clone(),
        }
    }
}

impl Source<16> for TestClientToServerSource
{
    fn poll(&mut self, buffer: &mut [u8; 16]) -> bool
    {
        match self.channel.try_recv()
        {
            Ok((frame, player_input)) =>
            {
                *(<&mut [u8; 4]>::try_from(&mut buffer[0..4]).unwrap()) = frame.to_le_bytes();
                *(<&mut [u8; 8]>::try_from(&mut buffer[4..12]).unwrap()) = player_input.to_le_bytes();
                true
            }
            Err(_) => false,
        }
    }
}

impl Sink<16> for TestClientToServerSink
{
    fn handle(&mut self, buffer: &[u8; 16])
    {
        let frame = u32::from_le_bytes(*(<&[u8; 4]>::try_from(&buffer[0..4]).unwrap()));
        let player_input = u64::from_le_bytes(*(<&[u8; 8]>::try_from(&buffer[4..12]).unwrap()));
        self.channel.send((frame, self.player_index, player_input)).unwrap();
    }
}

impl Source<32> for TestServerToClientSource
{
    fn poll(&mut self, buffer: &mut [u8; 32]) -> bool
    {
        match self.channel.try_recv()
        {
            Ok((frame, player_inputs)) =>
            {
                *(<&mut [u8; 4]>::try_from(&mut buffer[0..4]).unwrap()) = frame.to_le_bytes();
                *(<&mut [u8; 8]>::try_from(&mut buffer[4..12]).unwrap()) = player_inputs[0].to_le_bytes();
                *(<&mut [u8; 8]>::try_from(&mut buffer[12..20]).unwrap()) = player_inputs[1].to_le_bytes();
                true
            }
            Err(_) => false,
        }
    }
}

impl Sink<32> for TestServerToClientSink
{
    fn handle(&mut self, buffer: &[u8; 32])
    {
        let frame = u32::from_le_bytes(*(<&[u8; 4]>::try_from(&buffer[0..4]).unwrap()));
        let player_input_1 = u64::from_le_bytes(*(<&[u8; 8]>::try_from(&buffer[4..12]).unwrap()));
        let player_input_2 = u64::from_le_bytes(*(<&[u8; 8]>::try_from(&buffer[12..20]).unwrap()));
        self.channel.send((frame, [player_input_1, player_input_2])).unwrap();
    }
}

#[test]
fn golden()
{
    const TICK_PERIOD: u16 = 0;

    let client_to_server_mapper_socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
    let client_to_server_socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();

    let server_to_client_mapper_socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();

    let client_to_server_schema = ClientToServerSchema {
        name: "Input",

        mapper_port: client_to_server_mapper_socket.local_addr().unwrap().port(),
        heartbeat_period: 2000,

        port: client_to_server_socket.local_addr().unwrap().port(),
    };

    let server_to_client_schema = ServerToClientSchema {
        name: "State",

        mapper_port: server_to_client_mapper_socket.local_addr().unwrap().port(),
        heartbeat_period: 2000,
    };

    let sessions = [
        TestSession {
            session_id: 1,
            cipher_key: 0xDEADBEEFDEADBEEF,
        },
        TestSession {
            session_id: 2,
            cipher_key: 0xBEEFDEADBEEFDEAD,
        },
    ];

    let server_runtime = TestRuntime::new(TICK_PERIOD);
    let server_source_channels = [flume::unbounded(), flume::unbounded()];
    let server_sink_channel = flume::unbounded();

    let client_runtimes = [TestRuntime::new(TICK_PERIOD), TestRuntime::new(TICK_PERIOD)];
    let client_source_channels = [flume::unbounded(), flume::unbounded()];
    let client_sink_channels = [flume::unbounded(), flume::unbounded()];

    let mut server = Server::builder(Box::new(server_runtime.clone()), 2)
        .sender_with_sockets::<_, 32, 3>(
            &server_to_client_schema,
            server_to_client_mapper_socket,
            enum_map! {
                Mirroring::AudioVideo => UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap(),
                Mirroring::Background => UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap(),
                Mirroring::Voice => UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap(),
            },
            TestServerToClientSourceFactory {
                channels: [server_source_channels[0].1.clone(), server_source_channels[1].1.clone()],
            },
        )
        .unwrap()
        .receiver_with_socket::<_, 16, 3>(
            &client_to_server_schema,
            client_to_server_mapper_socket,
            client_to_server_socket,
            TestClientToServerSinkFactory {
                channel: server_sink_channel.0.clone(),
            },
        )
        .unwrap()
        .build();
    server.register(Box::new(sessions[0].clone()));
    server.register(Box::new(sessions[1].clone()));

    let _client_1 = Client::builder(Box::new(client_runtimes[0].clone()), Box::new(sessions[0].clone()))
        .sender::<_, 16, 3>(
            &client_to_server_schema,
            TestClientToServerSource {
                channel: client_source_channels[0].1.clone(),
            },
        )
        .unwrap()
        .receiver::<_, 32, 3>(
            &server_to_client_schema,
            TestServerToClientSink {
                channel: client_sink_channels[0].0.clone(),
            },
        )
        .unwrap()
        .build();

    let _client_2 = Client::builder(Box::new(client_runtimes[1].clone()), Box::new(sessions[1].clone()))
        .sender::<_, 16, 3>(
            &client_to_server_schema,
            TestClientToServerSource {
                channel: client_source_channels[1].1.clone(),
            },
        )
        .unwrap()
        .receiver::<_, 32, 3>(
            &server_to_client_schema,
            TestServerToClientSink {
                channel: client_sink_channels[1].0.clone(),
            },
        )
        .unwrap()
        .build();

    // Initial
    {
        assert!(server_sink_channel.1.is_empty());
        assert!(client_sink_channels[0].1.is_empty());
        assert!(client_sink_channels[1].1.is_empty());

        server_runtime.tick();
        client_runtimes[0].tick();
        client_runtimes[1].tick();
        assert!(server_sink_channel.1.is_empty());
        assert!(client_sink_channels[0].1.is_empty());
        assert!(client_sink_channels[1].1.is_empty());
    }

    // Client 1 input
    {
        client_source_channels[0].0.send((1, 10)).unwrap();
        client_runtimes[0].tick();

        server_runtime.tick();
        client_runtimes[0].tick();
        client_runtimes[1].tick();
        assert_eq!(server_sink_channel.1.try_recv().unwrap(), (1, 0, 10));
        assert!(server_sink_channel.1.is_empty());
        assert!(client_sink_channels[0].1.is_empty());
        assert!(client_sink_channels[1].1.is_empty());
    }

    // Client 2 input
    {
        client_source_channels[1].0.send((1, 20)).unwrap();
        client_runtimes[1].tick();

        server_runtime.tick();
        client_runtimes[0].tick();
        client_runtimes[1].tick();
        assert_eq!(server_sink_channel.1.try_recv().unwrap(), (1, 1, 20));
        assert!(server_sink_channel.1.is_empty());
        assert!(client_sink_channels[0].1.is_empty());
        assert!(client_sink_channels[1].1.is_empty());
    }

    // Server input
    {
        server_source_channels[0].0.send((1, [10, 20])).unwrap();
        server_source_channels[1].0.send((1, [10, 20])).unwrap();
        server_runtime.tick();

        server_runtime.tick();
        client_runtimes[0].tick();
        client_runtimes[1].tick();
        assert!(server_sink_channel.1.is_empty());
        assert_eq!(client_sink_channels[0].1.try_recv().unwrap(), (1, [10, 20]));
        assert!(client_sink_channels[0].1.is_empty());
        assert_eq!(client_sink_channels[1].1.try_recv().unwrap(), (1, [10, 20]));
        assert!(client_sink_channels[1].1.is_empty());
    }

    // All inputs (with double tick)
    {
        client_source_channels[0].0.send((2, 30)).unwrap();
        client_source_channels[1].0.send((2, 40)).unwrap();
        server_source_channels[0].0.send((3, [50, 60])).unwrap();
        server_source_channels[1].0.send((3, [50, 60])).unwrap();

        server_runtime.tick();
        client_runtimes[0].tick();
        client_runtimes[1].tick();
        server_runtime.tick();
        client_runtimes[0].tick();
        client_runtimes[1].tick();
        assert_eq!(server_sink_channel.1.try_recv().unwrap(), (2, 0, 30));
        assert_eq!(server_sink_channel.1.try_recv().unwrap(), (2, 1, 40));
        assert!(server_sink_channel.1.is_empty());
        assert_eq!(client_sink_channels[0].1.try_recv().unwrap(), (3, [50, 60]));
        assert!(client_sink_channels[0].1.is_empty());
        assert_eq!(client_sink_channels[1].1.try_recv().unwrap(), (3, [50, 60]));
        assert!(client_sink_channels[1].1.is_empty());
    }
}
