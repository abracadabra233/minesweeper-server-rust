### use axum::get

1. rust 是如何 实现 get(handler)中 handler 的多种重载的 ，即 ws_handler 可以只接受一个 WebSocketUpgrade 参数，也可以接受 Query 和 WebSocketUpgrade 参数 ；还可以是 Query，WebSocketUpgrade，HeaderMap 三个参数

- `axum::get` 可以接受多种`handle`函数，用于处理不同的请求，比如不同的参数，表单参数，body 参数，websocket 连接
- `axum::get`的参数其实是一个实现了 `Handler`特征的函数
- 通过宏为很多种 `handle`函数实现`Handler`特征
- `handle`函数的参数需要实现 `FromRequest`特征，只有参数满足这个条件的函数才会被视为一个 `Handler`
- `Handler` 会提供一个方法 `call`，这个方法会接受请求`req`和请求状态`state`
- 于是在 方法 `call`中，可以将请求`req`解析到不同的 `FromRequest`参数上，`handle`函数便可以直接来处理请求中得到的参数
- 关键结构体函数宏:
  - handler::Handler,
  - macro_rules! impl_handler
  - all_the_tuples!(impl_handler)
  - fn call(self, \_req: Request, \_state: S)

### 认识

1. 不能在对 结构体进行可变借用的过程中，移出改结构体字段的所有权，可以改为 Option.take()
