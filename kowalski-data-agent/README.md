# Kowalski Data Agent

A specialized AI agent for data analysis and processing tasks, built on top of the Kowalski framework. The Data Agent provides intelligent, conversational data analysis capabilities with support for structured data formats like CSV files.

## What is the Data Agent?

The Data Agent is a sophisticated AI-powered data analysis tool that combines the power of large language models with specialized data processing tools. It's designed to help users analyze, understand, and extract insights from structured data through natural language conversations.

### Core Capabilities

- **Intelligent Data Processing**: Automatically analyzes CSV files and other structured data formats
- **Statistical Analysis**: Provides comprehensive statistical summaries including averages, ranges, distributions, and correlations
- **AI-Powered Insights**: Uses advanced language models to generate human-readable insights and recommendations
- **Conversational Interface**: Supports interactive follow-up questions and iterative analysis
- **Streaming Responses**: Real-time processing and display of analysis results
- **Role-Based Analysis**: Configurable analysis styles based on user roles and requirements

## What Does It Do?

The Data Agent performs several key functions:

1. **Data Ingestion**: Reads and validates structured data from various sources
2. **Statistical Computation**: Calculates descriptive statistics for all data columns
3. **Pattern Recognition**: Identifies trends, outliers, and relationships in the data
4. **Insight Generation**: Provides contextual analysis and business recommendations
5. **Interactive Q&A**: Answers follow-up questions about specific aspects of the data
6. **Report Generation**: Creates comprehensive analysis reports with actionable insights

## Example Usage

### Basic CSV Analysis

```rust
use kowalski_data_agent::DataAgent;
use kowalski_core::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the data agent
    let config = Config::default();
    let mut data_agent = DataAgent::new(config).await?;
    
    // Start a conversation
    let conversation_id = data_agent.start_conversation("llama3.2");
    
    // Sample CSV data
    let csv_data = r#"name,age,city,salary,department
John Doe,30,New York,75000,Engineering
Jane Smith,28,San Francisco,85000,Marketing
Bob Johnson,35,Chicago,65000,Sales"#;
    
    // Process the CSV data
    let analysis_result = data_agent.process_csv(csv_data).await?;
    
    // Get AI analysis
    let analysis_prompt = format!(
        "Please analyze this CSV data and provide insights:\n\n{}\n\nAnalysis results:\n{}",
        csv_data,
        serde_json::to_string_pretty(&analysis_result.summary)?
    );
    
    let response = data_agent
        .chat_with_history(&conversation_id, &analysis_prompt, None)
        .await?;
    
    // Process streaming response...
    Ok(())
}
```

### Advanced Analysis with Custom Roles

```rust
use kowalski_core::role::{Audience, Preset, Role};

// Create a specialized role for business analysis
let role = Role::new(
    "Business Data Analyst",
    "You are an expert business analyst specializing in HR and financial data.",
)
.with_audience(Audience::new(
    "HR Manager",
    "You are speaking to an HR manager who needs actionable insights for decision-making.",
))
.with_preset(Preset::new(
    "Strategic Analysis",
    "Focus on strategic implications and actionable recommendations.",
));

// Use the role in analysis
let response = data_agent
    .chat_with_history(&conversation_id, &analysis_prompt, Some(role))
    .await?;
```

### Follow-up Questions

```rust
// Ask specific follow-up questions
let follow_up = "What are the key insights about salary distribution across departments?";
let follow_up_response = data_agent
    .chat_with_history(&conversation_id, follow_up, None)
    .await?;
```

## How Could It Be Extended?

The Data Agent is designed with extensibility in mind and can be enhanced in several ways:

### 1. Additional Data Formats

```rust
// Add support for JSON, XML, Excel files
pub struct ExtendedDataAgent {
    agent: DataAgent,
    json_tool: JsonTool,
    excel_tool: ExcelTool,
    xml_tool: XmlTool,
}
```

### 2. Advanced Analytics

```rust
// Integrate with statistical libraries
use statrs::statistics::Statistics;

pub struct AdvancedDataAgent {
    agent: DataAgent,
    correlation_analyzer: CorrelationAnalyzer,
    outlier_detector: OutlierDetector,
    trend_analyzer: TrendAnalyzer,
}
```

### 3. Machine Learning Integration

```rust
// Add ML capabilities for predictive analysis
pub struct MLDataAgent {
    agent: DataAgent,
    prediction_engine: PredictionEngine,
    clustering_analyzer: ClusteringAnalyzer,
    anomaly_detector: AnomalyDetector,
}
```

### 4. Visualization Tools

```rust
// Generate charts and graphs
pub struct VisualDataAgent {
    agent: DataAgent,
    chart_generator: ChartGenerator,
    plot_creator: PlotCreator,
    dashboard_builder: DashboardBuilder,
}
```

### 5. Database Integration

```rust
// Connect to databases for live data analysis
pub struct DatabaseDataAgent {
    agent: DataAgent,
    sql_connector: SqlConnector,
    nosql_connector: NoSqlConnector,
    query_optimizer: QueryOptimizer,
}
```

### 6. Real-time Data Processing

```rust
// Handle streaming data sources
pub struct StreamingDataAgent {
    agent: DataAgent,
    stream_processor: StreamProcessor,
    window_analyzer: WindowAnalyzer,
    alert_system: AlertSystem,
}
```

## Potential Benefits of Using It

### For Data Scientists
- **Rapid Prototyping**: Quickly analyze new datasets without writing custom code
- **Exploratory Data Analysis**: Automated initial insights and pattern discovery
- **Documentation**: Natural language explanations of complex statistical findings
- **Collaboration**: Share insights with non-technical stakeholders

### For Business Analysts
- **Accessibility**: No coding required for sophisticated data analysis
- **Speed**: Instant insights from data without waiting for custom reports
- **Flexibility**: Ask follow-up questions and drill down into specific areas
- **Actionable Insights**: Business-focused recommendations and implications

### For Organizations
- **Cost Reduction**: Reduce time spent on routine data analysis tasks
- **Improved Decision Making**: Data-driven insights accessible to all stakeholders
- **Scalability**: Handle multiple datasets and analysis requests simultaneously
- **Knowledge Democratization**: Make data analysis accessible to non-technical users

### For Developers
- **Integration**: Easy to embed in existing applications and workflows
- **Customization**: Extensible architecture for domain-specific analysis
- **API-First**: RESTful interface for programmatic access
- **Streaming**: Real-time analysis capabilities for live data

## Features

- Demonstrates CSV processing with sample employee data
- Shows statistical analysis results
- Interactive AI analysis with role-based prompts
- Follow-up questions for deeper insights
- Proper error handling and streaming response processing

The CSV tool can be easily extended to support other data formats or additional analysis features.

## Output of csv_analysis example

The example successfully:
- Processes CSV data with 10 employee records
- Generates statistical summaries (age, salary, department analysis)
- Provides AI-powered insights about the data
- Handles follow-up questions about salary distribution
- Shows proper tool integration and streaming responses

Running `/opt/ml/kowalski/target/debug/examples/csv_analysis`

-----
📊 Starting CSV Analysis...
Data Agent Conversation ID: 9bed7077-5a3d-4eae-a5f4-5f52efac9c3c

📈 Processing CSV Data:
```csv
name,age,city,salary,department
John Doe,30,New York,75000,Engineering
Jane Smith,28,San Francisco,85000,Marketing
Bob Johnson,35,Chicago,65000,Sales
Alice Brown,32,Boston,70000,Engineering
Charlie Wilson,29,Seattle,80000,Engineering
Diana Davis,31,Austin,72000,Marketing
Eve Miller,27,Denver,68000,Sales
Frank Garcia,33,Portland,75000,Engineering
Grace Lee,26,Atlanta,65000,Marketing
Henry Taylor,34,Dallas,78000,Engineering
```

📊 CSV Analysis Results:
```
Headers: ["name", "age", "city", "salary", "department"]
Total Rows: 10
Total Columns: 5
Summary: {
  "column_count": 5,
  "columns": {
    "age": {
      "average": 30.5,
      "count": 10,
      "max": 35.0,
      "min": 26.0,
      "sum": 305.0,
      "type": "numeric"
    },
    "city": {
      "count": 10,
      "most_common": "Boston",
      "most_common_count": 1,
      "type": "text",
      "unique_count": 10
    },
    "department": {
      "count": 10,
      "most_common": "Engineering",
      "most_common_count": 5,
      "type": "text",
      "unique_count": 3
    },
    "name": {
      "count": 10,
      "most_common": "John Doe",
      "most_common_count": 1,
      "type": "text",
      "unique_count": 10
    },
    "salary": {
      "average": 73300.0,
      "count": 10,
      "max": 85000.0,
      "min": 65000.0,
      "sum": 733000.0,
      "type": "numeric"
    }
  },
  "row_count": 10
}
```

🤖 AI Analysis:
**Comprehensive Analysis of the CSV Data**

After analyzing the provided CSV data, I have identified several key insights and recommendations.

**Summary Statistics:**

The dataset contains a total of 10 records, each representing an employee's information. The summary statistics for each column are as follows:

*   **Age**: The average age is 30.5 years, with a range of 26.0 to 35.0 years. This suggests that the employees are relatively young, with most being in their late 20s and early 30s.
*   **City**: There are 10 unique cities represented, with Boston being the most common city (1 occurrence). This indicates that many employees are based in Boston, but other cities also have a significant presence.
*   **Department**: The most common department is Engineering (5 occurrences), followed by Marketing (3 occurrences) and Sales (2 occurrences). This suggests that Engineering is a dominant function within the organization.
*   **Name**: Each employee has a unique name, with John Doe being the most common name (1 occurrence). This indicates that individuality is valued in the organization.
*   **Salary**: The average salary is $73,300.00, with a range of $65,000.00 to $85,000.00. This suggests that salaries are generally competitive within the industry.

**Insights:**

Based on these summary statistics and the data itself, several insights can be drawn:

*   **Demographics**: The employees appear to be relatively young, with most being in their late 20s and early 30s.
*   **Location**: Boston is a significant hub for the organization, followed by other cities with smaller but still notable presences.
*   **Function**: Engineering is a dominant function within the organization, suggesting that technical expertise is highly valued.
*   **Individuality**: Each employee has a unique name, indicating that individuality is encouraged and appreciated.

**Recommendations:**

Based on these insights, several recommendations can be made:

*   **Talent Acquisition**: Consider attracting more young talent to fill potential gaps in the engineering department. This could involve targeting recent graduates or individuals with early-career experience.
*   **Location Strategy**: Continue to prioritize Boston as a key location for the organization, but also explore opportunities to expand into other cities and regions.
*   **Function Development**: Invest in training and development programs to support the growth of engineers and other technical staff. This could include workshops, conferences, or online courses.

**Future Analysis:**

To further analyze this data, several options can be considered:

*   **Time Series Analysis**: Analyze the salary data over time to identify trends and patterns.
*   **Correlation Analysis**: Examine correlations between different columns (e.g., age vs. salary) to identify potential relationships.
*   **Machine Learning**: Use machine learning algorithms to predict employee salaries or job satisfaction based on their demographic and organizational characteristics.

By exploring these additional analysis options, further insights can be gained into the organization's dynamics and inform data-driven decisions.
✅ Analysis complete!


🔍 Follow-up Analysis:
**Salary Distribution Across Departments: Key Insights**

Analyzing the salary data reveals some interesting patterns:

*   **Engineering Department**: The most common salary range in the Engineering department is $65,000 to $80,000. This suggests that engineers in this role are generally well-compensated and have a strong career foundation.
*   **Marketing Department**: In contrast, the Marketing department has a more dispersed salary range, with some employees earning as little as $55,000 and others as much as $90,000. This may indicate that marketing roles are more variable in terms of compensation, possibly due to factors like performance-based bonuses or variable pay structures.
*   **Sales Department**: Sales employees have an even wider salary range, spanning from $50,000 to $100,000+. This could be attributed to the potential for high-commission sales roles, which can significantly impact individual earnings.

**Key Observations:**

1.  **Engineering department salaries are generally higher** across most ages and cities.
2.  **Marketing department salaries are more variable**, with some employees earning lower or higher than their colleagues in the same role.
3.  **Sales department salaries have a wide range**, potentially due to commission-based pay structures.

**Recommendations:**

*   **Engineering department**: Consider adjusting salary ranges for engineers to reflect industry standards and internal equity.
*   **Marketing department**: Examine performance-based bonuses or variable pay structures to ensure fair compensation and align with individual performance.
*   **Sales department**: Review commission structures and adjust as needed to ensure fair compensation and alignment with organizational goals.

**Future Analysis:**

To further explore salary distribution across departments, consider the following:

1.  **Correlation analysis**: Examine correlations between salary ranges and departmental variables like job responsibility, experience level, or industry standards.
2.  **Time series analysis**: Analyze changes in salary distributions over time to identify trends and potential factors influencing compensation decisions.
3.  **Regression analysis**: Use regression models to investigate the relationships between individual employee characteristics (e.g., age, location) and their salaries within specific departments.

By analyzing these additional metrics, deeper insights can be gained into the complex relationships between departmental variables and salary distributions.

------
