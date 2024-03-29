import argparse
import json
import threading

import websocket


def on_message(ws, message):
    print(f"Received message: {message}")
    try:
        response = json.loads(message)
        # Handle different types of messages based on the `type` field
        if response["type"] == "InitRoom":
            print(f"Room initialized with ID: {response['room_id']}")
        elif response["type"] == "JoinRoom":
            print("Joined room.")
        # Add handling for other message types as necessary
    except json.JSONDecodeError:
        print("Error decoding JSON from message")


def on_error(ws, error):
    print(f"Error: {error}")


def on_close(ws, close_status_code, close_msg):
    print("### closed ###")


def on_open(ws):
    def run(*args):
        if args[0] == "init":
            create_room_msg = json.dumps(
                {
                    "type": "InitRoom",
                    "player": {
                        "id": "id",
                        "name": "name",
                        "icon": "icon",  # 玩家头像的base64编码
                    },
                    "config": {
                        "cols": 10,
                        "rows": 10,
                        "mines": 16,
                    },
                }
            )
            ws.send(create_room_msg)
        else:
            join_room_msg = json.dumps(
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
            ws.send(join_room_msg)

        # Add any additional operations here if needed

    thread = threading.Thread(target=run, args=(args.flag,))
    thread.start()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--flag", type=str, help="init to create room, any other value to join room", default="init")
    args = parser.parse_args()

    websocket.enableTrace(True)
    url = "ws://abracadabra.v2.idcfengye.com/ws"
    url = "wss://lvpy.chailab.cn:33000/ws/mpm/ws"
    url = "ws://10.4.208.55:30081"
    url = "ws://10.4.208.55:8003/mpm"
    url = "wss://lvpy.chailab.cn:33000/mpm"
    ws = websocket.WebSocketApp(url, on_open=on_open, on_message=on_message, on_error=on_error, on_close=on_close)
    ws.run_forever()
