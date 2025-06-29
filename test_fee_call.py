
import sys
import json
import base64
import requests

def call_get_fee_info():
    # Instruction 21 = GetFeeInfo
    instruction_data = base64.b64encode(bytes([21])).decode()
    
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": [
            {
                "instructions": [
                    {
                        "programId": "4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn",
                        "accounts": [
                            {"pubkey": "5ZXXVaaFWRxpEaNyc5n1iE7K5cNGN6tRSAcZ6Apji1vG", "isSigner": False, "isWritable": False}
                        ],
                        "data": instruction_data
                    }
                ],
                "signers": []
            },
            {"encoding": "base64"}
        ]
    }
    
    response = requests.post("http://192.168.9.81:8899", json=payload)
    result = response.json()
    
    print("=== GetFeeInfo Response ===")
    print(json.dumps(result, indent=2))
    
    if "result" in result and "value" in result["result"]:
        if "logs" in result["result"]["value"]:
            print("
=== Program Logs ===")
            for log in result["result"]["value"]["logs"]:
                print(log)

call_get_fee_info()

