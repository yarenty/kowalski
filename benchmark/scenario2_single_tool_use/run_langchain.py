import time
from langchain_ollama import OllamaLLM
from langchain_core.tools import Tool
from langchain.agents import AgentExecutor, create_react_agent
from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder
from langchain.prompts import PromptTemplate

# Define a dummy tool for benchmarking
def get_first_lines_of_file(file_path: str, num_lines: int = 10) -> str:
    """Returns the first N lines of a given file."""
    try:
        with open(file_path, 'r') as f:
            lines = [f.readline() for _ in range(num_lines)]
        return "\n".join(line.strip() for line in lines)
    except FileNotFoundError:
        return f"Error: File not found at {file_path}"
    except Exception as e:
        return f"Error reading file: {e}"

fs_tool = Tool(
    name="fs_tool",
    func=get_first_lines_of_file,
    description="Useful for reading the first N lines of a file. Input should be a file_path and optionally num_lines."
)

def main():
    llm = OllamaLLM(model="llama3.2")
    tools = [fs_tool]

    template = """Answer the following questions as best you can. You have access to the following tools:

{tools}

Use the following format:

Question: the input question you must answer
Thought: you should always think about what to do
Action: the action to take, should be one of [{tool_names}]
Action Input: the input to the action
Observation: the result of the action
... (this Thought/Action/Action Input/Observation can repeat N times)
Thought: I now know the final answer
Final Answer: the final answer to the original input question

Begin!

Question: {input}
{agent_scratchpad}"""

    prompt = PromptTemplate(
        input_variables=["input", "tools", "tool_names", "agent_scratchpad"],
        template=template,
    )

    agent = create_react_agent(llm, tools, prompt)
    agent_executor = AgentExecutor(agent=agent, tools=tools, verbose=False)

    start_time = time.time()
    response = agent_executor.invoke({
        "input": "Get the first 10 lines of example.txt"
    })
    end_time = time.time()
    elapsed = end_time - start_time

    print(f"LangChain (FS Tool Use) - Response: {response['output']}")
    print(f"LangChain (FS Tool Use) - Time: {elapsed:.4f} seconds")

if __name__ == "__main__":
    main()