# 导入必要的库
import asyncio
import json
import traceback
from urllib.parse import quote_plus, urlencode

import websockets


async def send(websocket):
    create_room_msg = json.dumps({"type": "create_room"})
    await websocket.send(create_room_msg)
    asyncio.sleep(500)
    await websocket.close()


async def recv(websocket):
    try:
        while True:
            response = await websocket.recv()
            room_id = json.loads(response).get("room_id")
            print(f"创建房间响应: {response},{room_id}")

    except Exception as e:
        traceback.print_exc()
        # await websocket.close()
        exit()


async def ws_client():
    user_id = "5555"
    user_name = "kkkkk"
    user_icon = "image8888"

    params = {"room_id": "123456", "player": {"user_name": "user01", "user_name": "Alice", "user_icon": "avatarStringBase64"}}
    params_json = json.dumps(params)
    encoded_params = urlencode({"params": params_json}, quote_via=quote_plus)
    uri = f"ws://127.0.0.1:3000/ws?{encoded_params}"
    headers = {
        "Authorization": "Basic YWxhZGRpbjpvcGVuc2VzYW1l",
        "Custom-Header": "Value",
    }

    async with websockets.connect(uri, subprotocols=["binary"], ping_interval=None, extra_headers=headers) as websocket:
        task1 = asyncio.create_task(send(websocket))
        task2 = asyncio.create_task(recv(websocket))
        await asyncio.gather(task1, task2)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(ws_client())
    asyncio.get_event_loop().run_forever()
