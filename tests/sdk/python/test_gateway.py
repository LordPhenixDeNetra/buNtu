"""Preuves SDK OpenAI (Python) contre buNtu + simulateur (T1.13).

Le SDK officiel `openai` est utilisé sans modification : seule l'URL de base change.
"""

import os

from openai import OpenAI

BASE = os.environ["BUNTU_BASE_URL"]
client = OpenAI(base_url=f"{BASE}/v1", api_key="sk-test")

MESSAGES = [{"role": "user", "content": "bonjour"}]


def test_chat_text():
    completion = client.chat.completions.create(model="chat", messages=MESSAGES)
    assert completion.choices[0].message.content
    assert completion.usage.prompt_tokens > 0


def test_chat_stream():
    chunks = client.chat.completions.create(model="chat", messages=MESSAGES, stream=True)
    text = "".join(
        chunk.choices[0].delta.content or ""
        for chunk in chunks
        if chunk.choices and chunk.choices[0].delta.content
    )
    assert text.strip()


def test_tools_are_accepted():
    completion = client.chat.completions.create(
        model="chat",
        messages=MESSAGES,
        tools=[
            {
                "type": "function",
                "function": {
                    "name": "meteo",
                    "description": "Météo d'une ville",
                    "parameters": {
                        "type": "object",
                        "properties": {"ville": {"type": "string"}},
                    },
                },
            }
        ],
    )
    assert completion.choices


def test_embeddings():
    result = client.embeddings.create(model="chat", input="bonjour")
    assert len(result.data[0].embedding) > 0


def test_models_list():
    models = client.models.list()
    assert any(model.id == "chat" for model in models.data)
