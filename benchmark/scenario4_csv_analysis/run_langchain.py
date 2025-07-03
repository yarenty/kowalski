import time
import pandas as pd
from langchain_community.llms import Ollama
from langchain_experimental.agents import create_pandas_dataframe_agent

def main():
    llm = Ollama(model="llama3.2")

    csv_data = """name,age,city,salary,department
John Doe,30,New York,75000,Engineering
Jane Smith,28,San Francisco,85000,Marketing
Bob Johnson,35,Chicago,65000,Sales
Alice Brown,32,Boston,70000,Engineering
Charlie Wilson,29,Seattle,80000,Engineering
Diana Davis,31,Austin,72000,Marketing
Eve Miller,27,Denver,68000,Sales
Frank Garcia,33,Portland,75000,Engineering
Grace Lee,26,Atlanta,65000,Marketing
Henry Taylor,34,Dallas,78000,Engineering"""

    # Create a DataFrame from the CSV data
    df = pd.read_csv(pd.io.common.StringIO(csv_data))

    # Create the pandas dataframe agent
    agent = create_pandas_dataframe_agent(llm, df, verbose=False)

    start_time = time.time()
    response = agent.invoke({"input": "Analyze this data and provide key insights about salaries and departments."})
    end_time = time.time()
    elapsed = end_time - start_time

    print(f"LangChain (CSV Analysis) - Response: {response['output']}")
    print(f"LangChain (CSV Analysis) - Time: {elapsed:.4f} seconds")

if __name__ == "__main__":
    main()
