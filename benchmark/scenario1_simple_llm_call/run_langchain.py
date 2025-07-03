import time
from langchain_ollama import OllamaLLM
from langchain_core.prompts import ChatPromptTemplate

def main():
    llm = OllamaLLM(model="llama3.2")
    prompt = ChatPromptTemplate.from_messages([("user", "{input}")])
    chain = prompt | llm

    start_time = time.time()
    response = chain.invoke({"input": "Tell me a short joke."})
    end_time = time.time()
    elapsed = end_time - start_time

    print(f"LangChain (Simple LLM Call) - Response: {response}")
    print(f"LangChain (Simple LLM Call) - Time: {elapsed:.4f} seconds")

if __name__ == "__main__":
    main()
