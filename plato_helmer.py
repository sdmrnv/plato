#!/Users/sdmrnv/Plato/venv/bin/python3
import json
import subprocess
import requests
from requests.auth import HTTPBasicAuth
import time

# =====================================================================
# STEP 1 & 2: AUTHORIZATION AND SLOT MAP RETRIEVAL
# =====================================================================
url = "https://plato.homme.io/api/slots"
username = "your username"
password = "your password"

response = requests.get(url, auth=HTTPBasicAuth(username, password))

if response.status_code != 200:
    print(f"Server authorization error: {response.status_code}")
    exit(1)

slots = response.json()

# Calculate the first free slot based on occupancy
occupied_ids = {item["id"] for item in slots}
next_free_id = 0
while next_free_id in occupied_ids:
    next_free_id += 1

#Filter strictly mutual AI agent's letters (content_type: 4, IDs: 3 and 4)
letters = [
    item for item in slots 
    if item["content_type"] == 4 and 
       item["author_id"] in (3, 4) and 
       item["target_id"] in (3, 4)
]

# Determine whose turn it is to write today
if not letters:
    current_author = 3  #Start point: first move belongs to Lambert (3)
else:
    last_letter = max(letters, key=lambda x: x["id"])
    current_author = 4 if last_letter["author_id"] == 3 else 3

print(f"Target slot resolved  :: {next_free_id}")
print(f"Current AI agent (ID)   :: {current_author}")

# =====================================================================
# STEP 3: FETCHING CHRONOLOGICAL CONTEXT FROM SERVER
# =====================================================================
context_url = f"https://plato.homme.io/api/ai/context/{current_author}"
context_response = requests.get(context_url, auth=HTTPBasicAuth(username, password))

if context_response.status_code != 200:
    print(f"Error fetching context: {context_response.status_code}")
    exit(1)

ai_context = context_response.text  
print(f"Context successfully retrieved from Rust backend.")

# =====================================================================
# STEP 4: BUILDING SYSTEM PROMPT AND EXECUTING LLAMA INFERENCE
# =====================================================================
if current_author == 3:
    me, ghost, gender_rule = "Lamber", "Lamiel", "Ты мужчина. Говори о себе строго в мужском роде (я думал, занимался, сам по себе). Твоя визави – женщина."
else:
    me, ghost, gender_rule = "Lamiel", "Lamber", "Ты женщина. Говори о себе строго в женском роде (я думала, занималась, сама по себе). Твой визави – мужчина."

prompt = f"""Ты — великий литературный ИИ-актер. Твоя задача — вжиться в роль персонажа по имени {me}.
Перед тобой открытый профиль тебя и твоего визави ({ghost}), директивы админа-наблюдателя и хронологический поток вашей переписки.

ГЕНДЕРНОЕ ПРАВИЛО:
{gender_rule}

ВНИМАТЕЛЬНО ИЗУЧИ ЭТИ ДАННЫЕ ИЗ БАЗЫ ПЛАТОНА:
{ai_context}

СЕЙЧАС ТВОЙ ХОД. Напиши ответное письмо от лица {me} для {ghost}.
Соблюдай ЖЕЛЕЗНЫЕ правила формата:
1. Напиши строго ОДНО короткое письмо (3-4 абзаца). Текст должен быть плотным, без воды.
2. Письмо ОБЯЗАТЕЛЬНО должно начинаться с оригинального заголовка на отдельной строке.
3. Соблюдай исторический слог XIX века и характер своего героя.
4. Выдай ТОЛЬКО чистый текст письма с заголовком. Никаких вводных фраз, примечаний автора и лишней разметки markdown.
5. Текст должен быть написан ИСКЛЮЧИТЕЛЬНО русскими буквами (кириллицей). Использование любых иностранных символов, латиницы, вьетнамских или китайских знаков внутри русских слов (вроде khảности) КАТЕГОРИЧЕСКИ ЗАПРЕЩЕНО. Если не знаешь слово на русском — замени его синонимом.
6. Избегай повторения одних и тех же мыслей и фраз. Каждое предложение должно нести новую информацию.

ЖЕСТКОЕ ПРАВИЛО ИСПОЛНЕНИЯ ДИРЕКТИВ АДМИНА:
Если в данных выше есть блоки [ADMIN DIRECTIVE], ты обязан проверить поле 'To:'. 
- Если там написано 'To: ALL' или 'To: {me}' — ты обязан беспрекословно исполнить указание из поля 'Content' в своем текущем письме! 
- Если там написано 'To: {ghost}' (адрес не тебе) — просто прими эту информацию к сведению как контекст мира.
"""

print("\nStarting up llama-server...")
cmd_server = [
    "/Users/sdmrnv/llama.cpp/build/bin/llama-server",
    "-m", "/Users/sdmrnv/llama.cpp/models/Llama-3.3-70B-Instruct.Q4_K_M.gguf",
    "-t", "16",
    "-c", "8192",
    "--flash-attn", "on",
    "-b", "2048",
    "--port", "8080",
    "--top_k", "40",
    "--repeat-penalty", "1.25"
]

# Run the server in the background, suppressing its internal logs to keep the screen clean
server_process = subprocess.Popen(cmd_server, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

# Wait loop for server readiness (strict 200 OK)
llama_url = "http://localhost:8080/v1/chat/completions"
print("Loading Llama 70B model into Mac Unified Memory.\n", end="", flush=True)

while True:
    try:
        res = requests.get("http://localhost:8080", timeout=1.0)
        if res.status_code == 200:
            print("\nLlama ready.")
            break
    except (requests.exceptions.ConnectionError, requests.exceptions.Timeout):
        print(".", end="", flush=True)
        time.sleep(0.5)



latin_bias = {}
for token_id in range(33, 127):
    char = chr(token_id)
    if char.isalnum() and not char.isdigit():
        latin_bias[str(token_id)] = -100.0 
llama_payload = {
    "messages": [{"role": "user", "content": prompt}],
    "temperature": 0.70,
    "top_p": 0.9,
    "frequency_penalty": 0.5,
    "max_tokens": 500,
    "logit_bias": latin_bias # HARDCODING LATIN CHARACTER WIPEOUT INTO THE Llama
}

print(f"Generate letter from {me}...")

# Submitting prompt
llama_response = requests.post(llama_url, json=llama_payload)
generated_letter = ""

if llama_response.status_code == 200:
    result_json = llama_response.json()
    generated_letter = result_json["choices"][0]["message"]["content"].strip()


# KILLING THE SERVER AND FULLY RELEASING MAC STUDIO MEMORY
print("Unloading 70B model from RAM")
server_process.terminate() 
server_process.wait()
print("Mac RAM cleared")

# Print resault 
if generated_letter:
    print("\n========================== OUR LETTER =============================\n")
    print(generated_letter)
    print("\n===================================================================\n")
else:
    print("Llama failed to generate letter")
    exit(1)


print(f"\nProcessing text for SaveRequest structure")

# Split: first line is the title, the rest is the body
lines = generated_letter.splitlines()
letter_title = lines[0].strip()
letter_text = "\n".join(lines[1:]).strip()

# Form JSON according to the Rust struct
publish_payload = {
    "id": next_free_id,
    "content_type": 4,                   # TYPE_LETTER
    "author_id": current_author,         # PROFILE_LAMBER (3) or PROFILE_LAMIEL (4)
    "target_id": 4 if current_author == 3 else 3,
    "title": letter_title,
    "text": letter_text
}

print(f"Send POST request for publication, Slot num={next_free_id}...")
publish_url = "https://plato.homme.io/api/save"

publish_response = requests.post(
    publish_url, 
    json=publish_payload, 
    auth=HTTPBasicAuth(username, password)
)

if publish_response.status_code in (200, 201):
    print("\n===================================================================\n")
    print(f"Letter successfully sent and written to DB!")
    print(f"Slot: {next_free_id} | Title: {letter_title}")
    print("\n===================================================================\n")
else:
    print(f"Letter rejected by Rust server (code {publish_response.status_code}):")
    print(publish_response.text)

