<!-- post: date=2026-09-26; summary=Kowalski 2.1 hands three chores to a horde of small AI agents: questions about your spreadsheets, a morning brief, and the paperwork in your inbox. The numbers come from a query engine, not from the AI. -->
# Ask your spreadsheets. Get a workbook back. No number made up.

*September 2026*

If you run a small business, you have probably tried asking an AI chatbot about your sales
spreadsheet. It answers instantly, sounds sure of itself, and now and then gets a total wrong in a
way you only notice when a customer does. Along the way, you uploaded your customer list to
someone else's server.

Kowalski 2.1 is our answer to both problems. It is a free, open-source app that runs on your own
Mac or Linux machine and hands everyday chores to a **horde**, a small team of AI agents that each
do one job. Three hordes cover the chores we hear about most.

## 1. "Who were our best customers last quarter?"

You load your workbook into [tableski](https://tableski.io), which turns every sheet into a table,
open Kowalski, pick the **Spreadsheet analyst** and type your questions, one per line:

> Who spent the most in total across the orders?
>
> How many people are active?
>
> How many people live in each city?

A few minutes later you have two files. `report.xlsx` has an index sheet and one sheet per
question, with real numbers you can sort and chart. `HANDOFF.md` has the short answers:

> **Who spent the most?** Ada spent the most, with a total of 150.5, followed by Linus with 99.99.
>
> **How many people are active?** There are 2 active people.
>
> **People per city:** not answerable from these tables. The data has no city column.

That last answer matters as much as the first two. The AI did not invent a city breakdown to be
helpful. It said the data cannot answer the question, and why.

### Why the numbers can be trusted

Six agents take part, and **four of them use no AI at all**. One reads your tables' columns and
types, another runs the queries, another builds the workbook. The AI does two things: it turns
each question into a SQL query, and afterwards it explains the results it is handed. It never
adds up a column itself. Every figure in the workbook came out of a database engine, and the exact
query behind each answer is saved next to it, in case your accountant asks.

## 2. A morning brief from the pages you already check

Most of us open the same five tabs every morning: a news site, a competitor's price list, a
supplier's announcements, an industry newsletter. The **Morning brief** horde reads them for you
and writes one page: a top pick, then the few items from each page that match what you told it
you care about, each with its link.

It can run by itself at 7:00 on weekdays. That schedule ships switched off. Nothing runs, or
costs anything, until you turn it on.

## 3. The paperwork in your inbox

Point the **Folder watcher** at an inbox folder and drop documents into it: invoices, order
confirmations, a contract. For each one you get a short note. Here is the note for a supplier
invoice from our testing:

> **Invoice from ACME Packaging Ltd, September 2026**
>
> *Key facts:* 2,000 boxes at €0.62; VAT €285.20; **total due €1,525.20**.
>
> *What to do:* pay €1,525.20 by 17 October 2026.
>
> *Watch out:* a 5% surcharge after 17 October, and the unit price rises to €0.71 on 1 November.

The price rise was one line at the bottom of the invoice. That is the line you would have missed.

## Your machine, your files, your choice of AI

- **It runs on your computer.** Your spreadsheets and documents stay on your disk. Kowalski only
  talks to the AI model you choose and the pages you ask it to read.
- **Free, or bring your own key.** With [Ollama](https://ollama.com) the AI runs on your machine
  at no cost (a recent laptop is enough; a bigger one is faster). Or connect any OpenAI-compatible
  service with your own key and pay only for what you use.
- **It asks before it acts.** Any step that would run a command or change files stops and shows
  you what it is about to do. It continues only when you press Approve.
- **You can read everything it does.** Each horde is a folder of plain text files, and every run
  leaves its working files behind: the questions, the queries and the raw results.

## Two lines to try it

On a Mac or Linux machine, open the Terminal and paste:

```bash
curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash
kowalski
```

Your browser opens on a setup screen with three questions: which AI to use, which folder the
agents may work in, and (optional) your tableski account. Then pick a horde and give it
something to do.

Kowalski is open source under the MIT licence, at
[github.com/yarenty/kowalski](https://github.com/yarenty/kowalski). If a chore of yours would
make a good fourth horde, tell us. That is how these three were chosen.
