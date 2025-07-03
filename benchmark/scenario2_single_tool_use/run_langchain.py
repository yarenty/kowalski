import time
from langchain_community.llms import Ollama
from langchain_core.tools import Tool
from langchain.agents import AgentExecutor, create_react_agent
from langchain_core.prompts import ChatPromptTemplate

# Define a dummy tool for benchmarking
def get_weather(location: str) -> str:
    """Returns the current weather for a given location."""
    # Simulate some work
    time.sleep(0.1)
    return f"The weather in {location} is sunny with 20°C."

weather_tool = Tool(
    name="weather_tool",
    func=get_weather,
    description="Useful for getting the current weather for a location."
)

def main():
    llm = Ollama(model="llama3.2")
    tools = [weather_tool]

    # Define the prompt for the ReAct agent
    prompt = ChatPromptTemplate.from_messages([
        ("system", "You are a helpful AI assistant."),
        ("human", "{input}"),
        ("placeholder", "{agent_scratchpad}")
    ])

    # Create the ReAct agent
    agent = create_react_agent(llm, tools, prompt)
    agent_executor = AgentExecutor(agent=agent, tools=tools, verbose=False)

    start_time = time.time()
    response = agent_executor.invoke({"input": "What's the weather in London?"})
    end_time = time.time()
    elapsed = end_time - start_time

    print(f"LangChain (Single Tool Use) - Response: {response['output']}")
    print(f"LangChain (Single Tool Use) - Time: {elapsed:.4f} seconds")

if __name__ == "__main__":
    main()
