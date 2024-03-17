1. rust 是如何 实现get(handler)中handler 的多种重载的 ，即 ws_handler 可以只接受一个 WebSocketUpgrade 参数，也可以接受 Query 和 WebSocketUpgrade 参数 ；还可以是 Query，WebSocketUpgrade，HeaderMap 三个参数
   
### todo
[] 将ResponseModel中的各种结构体的字段类型全部采用引用类型，比如box，避免一直clone
[] 使用多消费者多生产者 通道模型，不然不同的房间 需要同时广播时，由于所有房间使用一个全局变量来获取锁