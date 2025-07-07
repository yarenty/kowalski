import time
from langchain_ollama import OllamaLLM
from langchain_community.embeddings import OllamaEmbeddings
from langchain_community.vectorstores import Chroma
from langchain.chains import RetrievalQA
from langchain_core.documents import Document

def main():
    # Initialize embeddings and vector store
    embeddings = OllamaEmbeddings(model="llama3.2")
    
    # Create a dummy in-memory Chroma DB for benchmarking
    # In a real scenario, this would be persistent and pre-populated
    docs = [
        Document(page_content="Kowalski is a high-performance, Rust-based framework for building AI agents."),
        Document(page_content="Rust offers memory safety, concurrency without GIL, and high performance."),
    ]
    vectorstore = Chroma.from_documents(docs, embeddings)

    llm = OllamaLLM(model="llama3.2")
    qa_chain = RetrievalQA.from_chain_type(llm=llm, retriever=vectorstore.as_retriever())

    start_time = time.time()
    response = qa_chain.invoke({"query": "Tell me about the project Kowalski and its benefits."})
    end_time = time.time()
    elapsed = end_time - start_time

    print(f"LangChain (Memory Retrieval) - Response: {response['result']}")
    print(f"LangChain (Memory Retrieval) - Time: {elapsed:.4f} seconds")

if __name__ == "__main__":
    main()
