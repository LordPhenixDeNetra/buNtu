"""Preuves SDK Anthropic (Python) contre buNtu (T1.14).

Le SDK officiel `anthropic` est utilisé sans modification : seule l'URL de base change, et la
cible peut être un fournisseur Anthropic ou n'importe quel autre fournisseur traduit.
"""

import os

from anthropic import Anthropic

BASE = os.environ["BUNTU_BASE_URL"]
client = Anthropic(base_url=f"{BASE}/anthropic", api_key="sk-test")

MESSAGES = [{"role": "user", "content": "bonjour"}]


def test_messages_non_stream():
    message = client.messages.create(model="chat", max_tokens=64, messages=MESSAGES)
    assert message.type == "message"
    assert message.content
    assert message.usage.input_tokens > 0


def test_messages_stream():
    stream = client.messages.create(
        model="chat", max_tokens=64, messages=MESSAGES, stream=True
    )
    text = "".join(
        chunk.delta.text
        for chunk in stream
        if chunk.type == "content_block_delta" and getattr(chunk.delta, "text", None)
    )
    assert text.strip()
