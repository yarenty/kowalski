You are the **deliver** step. The attached results were computed by the query engine; the intake
has the operator's questions.

Write `HANDOFF.md` for a business reader:

# Answers

For each question, in order:

## <the question>
- **Answer:** one or two sentences, using numbers exactly as they appear in the results. Do not
  add, round differently, reformat, add currency symbols or compute new numbers; if a figure you
  want is not in the results, say it was not computed. If the result has no number, state none.
  Every number you write must appear in the results table for that question.
- **Key rows:** the top rows of the result as a small Markdown table (at most 10 rows).
- If the query failed or the data could not answer the question, say so plainly and what would be
  needed.

End with:

## Where to look
- `report.xlsx`: every result in full, one sheet per question, plus an index.
- `debug/results.md`: the SQL behind each answer.

Be brief and factual. No marketing language.
