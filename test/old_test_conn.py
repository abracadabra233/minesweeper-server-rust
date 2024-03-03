# 导入必要的库
import argparse
import asyncio
import json
import traceback
from urllib.parse import quote_plus, urlencode

import websockets


async def send_create(websocket):
    create_room_msg = json.dumps(
        {
            "type": "InitRoom",
            "player": {
                "user_id": "user_id1",
                "user_name": "user_name1",
                "user_icon": "user_icon1",
            },
            "config": {
                "cols": 10,
                "rows": 10,
                "mines": 16,
            },
        }
    )
    await websocket.send(create_room_msg)


async def send_join(websocket):
    create_room_msg = json.dumps(
        {
            "type": "JoinRoom",
            "room_id": "66666",
            "player": {
                "user_id": "user_id2",
                "user_name": "user_name2",
                "user_icon": "user_icon2",
            },
        }
    )
    await websocket.send(create_room_msg)


async def send(websocket):
    if args.flag == "init":
        await send_create(websocket)
    else:
        await send_join(websocket)
    await asyncio.sleep(500)


async def recv(websocket):
    try:
        while True:
            response = await websocket.recv()
            # room_id = json.loads(response).get("room_id")
            print(f"创建房间响应: {response}")

    except Exception as e:
        traceback.print_exc()
        # await websocket.close()
        exit()


async def ws_client():
    user_id = "5555"
    user_name = "kkkkk"
    user_icon = "image8888"

    query_params = f"user_id={user_id}&user_name={user_name}&user_icon={user_icon}"
    uri = f"ws://127.0.0.1:3000/ws?{query_params}"
    headers = {
        "Authorization": "Basic YWxhZGRpbjpvcGVuc2VzYW1l",
        "Custom-Header": "Value",
        "user_name": "llllll",
        "rows": 10,
    }

    async with websockets.connect(uri, subprotocols=["binary"], ping_interval=None, extra_headers=headers) as websocket:
        task1 = asyncio.create_task(send(websocket))
        task2 = asyncio.create_task(recv(websocket))
        await asyncio.gather(task1, task2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--flag", type=str, default="init")
    args = parser.parse_args()

    asyncio.get_event_loop().run_until_complete(ws_client())
    asyncio.get_event_loop().run_forever()

"""
使用 python 的 websocket 库来替换上面的websockets 库，同时修改以下地方
1. 在 websocket的 on open 函数中更具 argparse参数选择发送 InitRoom或者JoinRoom 信息
2. 根据下面的rust结构体解析接受到的信息，rust的发送方式为
        let respose_mes = serde_json::to_string(&respose).unwrap();
        ws_sender.send(Message::Text(respose_mes)).await.unwrap();

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseModel {
    JoinRoom {
        player: Player,
    },
    InitRoom {
        room_id: String,
    },
    GameStart {
        players: Vec<Player>,
        config: Gconfig,
    },
    GameOpRes {
        op_res: OpResult,
    },
    GameEnd {
        success: bool,
        scores: usize,
        duration: usize,
        steps: usize,
    },
    InvalidRequest {
        error: String,
    },
}

"""
