1. rust 是如何 实现get(handler)中handler 的多种重载的 ，即 ws_handler 可以只接受一个 WebSocketUpgrade 参数，也可以接受 Query 和 WebSocketUpgrade 参数 ；还可以是 Query，WebSocketUpgrade，HeaderMap 三个参数
   
### todo
- 优化
  - [] 将ResponseModel中的各种结构体的字段类型全部采用引用类型，比如box，避免一直clone
  - [] 使用多消费者多生产者 通道模型，不然不同的房间 需要同时广播时，由于所有房间使用一个全局变量来获取锁
  - [] 把各种错误也封装在一起；便于维护，代码结构更加清晰；每一种request或每一种respose都会对应几个错误
- 完善
  - 在游戏界面加入  玩家列表，如果退出则头像变灰
  - 游戏结束后，玩家重新置为未准备状态；给出两个按钮，在玩一遍（重新进入等待页面，等待其他玩家准备）和 改变难度（给出选择页面，选择好后玩家进入等待页面）
  - 
