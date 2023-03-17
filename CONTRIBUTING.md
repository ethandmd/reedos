# Contributions Welcome
If you are in CSCI 393: Operating Systems.

Submit an issue if you are looking for feedback on a proposal or notice bugs, etc. Or message us on slack/discord/etc.

## Workflow
1. Fork this repository
2. Create a new branch for your contribution and give it an appropriate name.
For example, if you are going to tackle an issue called: "We need this bug fix now #42", you might name your branch:
`42-we-need-this-bug-fix-now`.
3. Write your code. Figure out how to test your code. Test your code.
4. Create a pull request (PR) against the main branch.
5. Add TCCQ and ethandmd as reviewers.
6. Once approved, we will merge your PR.

## Style Points
We recommend you use a git pre-commit hook like [rusty-hook](https://github.com/swellaby/rusty-hook) to help you format, test, etc. At the time of
writing we are continuing to refine our code structure, and do not have any testing framework in place (you could send in a PR for this!), so 
it will be a gametime decision on if your feature 'works'.
Before PR to main, use `cargo clippy` and `rustfmt`.

Generally, we try to do the following:
- Each logical change is in its own commit.
- No commit should break. (If we had better tests, each commit should pass testing)
- Document liberally and succinctly. It's ok to include vowels and complete english words in variable names.
- Commit messages should be concise and use active voice.
- Include your references, reasoning, and notable features of your contribution in docs and your PR.
