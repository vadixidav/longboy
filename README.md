# longboy

Longboy is a planned rust implementation of the Bristlecone server protocols with a focus on speed, accuracy, and reliability. 

Longboy provides:  
- Basic session management
- A TCP\QUIC backhaul connection used for session negotiation, time synchronization, recovery, and key exchange
- Full support for session-based routing of input using bristlecone both up and down

It uses a timesync design based in part on GGPO:  
https://github.com/pond3r/ggpo/blob/master/src/lib/ggpo/timesync.cpp  
And on https://medium.com/@invicticide/accurately-syncing-unreals-network-clock-87a3f9262594  
  
However, the timesync design we use is based on a more modern understanding of the availability of extremely precise timekeeping at scale. Thanks to the massive drop in cost of atomic clocks, it's now trivial to equip all federated hosting sites with synced clocks. This allows a multi-point time sync, much like NTP, to be employed. 
