# GNU coreutils Equivalents for CATS Tools

This report maps the CATS (Coding Agent ToolS) tools to their closest GNU coreutils (and common Unix utilities) equivalents. It highlights example commands, notes about differences, and feasibility of straight replacement by coreutils.

> NOTE: Some CATS tools provide structured, LLM-friendly behavior (stateful operations, safety checks, language-aware edits) that have no direct one-to-one mapping in coreutils. Where a direct mapping isn't possible, guidance or approximate alternatives are provided.

| CATS Tool | Short Description | GNU coreutils / Unix Equivalent | Example Command | Replacement Feasibility & Notes |
|---|---:|---|---|---|
| open | Opens a file and displays a window of lines | sed (print range) or less (interactive pager) | sed -n 'START,ENDp' file.txt  OR  less file.txt | High — `sed` can print exact line windows; `less` offers interactive scrolling. LLM-friendly metadata (window state) is lost. |
| goto | Jump to specific line in current file view | sed or awk | sed -n 'LINEp' file.txt  OR  awk 'NR==LINE{print; exit}' file.txt | High — simple to reproduce for one-shot views. Stateful session-aware goto is not provided by coreutils. |
| scroll_up / scroll_down | Scroll viewing window | less (interactive) or adjust sed ranges | less file.txt  OR  sed -n 'START,ENDp' file.txt | Medium — interactive scrolling via `less` works; recreating a session with remembered window position requires orchestration outside coreutils. |
| find_file | Find files by name pattern | find | find . -name "*.rs" | High — direct mapping. Use -iname for case-insensitive. |
| search_file | Search for text within a single file | grep -n | grep -n "TODO" file.txt | High — direct mapping. Use PCRE (grep -P) where needed. |
| search_dir | Search for text across directory recursively | grep -R --line-number | grep -R --line-number "TODO" . | High — direct mapping; consider ripgrep (rg) for performance (non-coreutils). |
| create_file | Create a new file with content | printf/echo/tee/redirection | printf '%s\n' "content" > newfile.txt  OR  tee newfile.txt <<< "$CONTENT" | High — direct mapping. Ensure proper escaping and binary-safe writes. |
| replace_text | Replace text using search/replace pattern in files | sed -i (in-place) or perl -pi -e | sed -i 's/old/new/g' file.txt  OR  perl -pi -e 's/old/new/g' file.txt | High — sed -i works but beware of portability (BSD sed vs GNU sed) and regex escaping. perl is more robust. |
| insert_text | Insert text at a specific line | sed -i 'LINEi\text' | sed -i '10i\// New comment' file.rs | Medium — sed insertion works but requires careful escaping; complex multi-line inserts are trickier. |
| delete_text | Delete a range of lines | sed -i 'START,ENDd' | sed -i '15,20d' file.txt | High — straightforward. |
| delete_line | Delete a specific line | sed -i 'Nd' | sed -i '42d' file.txt | High — straightforward. |
| overwrite_file | Replace entire file contents | redirection or tee | printf '%s\n' "new content" > file.txt | High — direct mapping. |
| delete_function | Delete a Rust function by name (language-aware) | No direct coreutils equivalent; use awk/sed/perl with complex regex or a language parser | perl -0777 -pe 's/fn\s+name\s*\([^\)]*\)\s*\{.*?\n\}//gs' -i file.rs (fragile) | Low — removing a function reliably requires parsing (Rust syntax), which coreutils cannot safely do. Use a proper parser or language-aware tool (rustfmt-based AST tools, or a small rustc/RA-based script). |
| delete_path | Delete a file or directory | rm -rf | rm -rf path/to/target | High — direct mapping. Be careful with rm -rf risks. |
| move_path | Move or rename a file/directory | mv | mv src dest | High — direct mapping. |
| copy_path | Copy a file or directory | cp -r | cp -r src dest | High — direct mapping. For file metadata, use -p. |
| create_directory | Create a new directory | mkdir -p | mkdir -p path/to/dir | High — direct mapping. |
| run_command | Execute shell commands with timeout and validation | direct shell execution; use timeout(1) for time-limited runs (coreutils does not include timeout, but GNU coreutils usually available in coreutils package) | timeout 30s bash -c 'some_command' | Medium — running commands is native to the shell; CATS adds validation and structured outputs. For timeouts, use coreutils timeout (from coreutils package) or `timeout` provided by GNU coreutils. Validate/escape args for safety. |
| _state | Display current tool state and context (LLM-specific) | N/A | N/A | None — this is an agent-internal feature with no general shell equivalent. |
| count_tokens | Count tokens in a file (tiktoken/cl100k_base) | wc (words/bytes/lines) as an approximation | wc -w file.txt  OR  wc -c file.txt | Low — tokenization for LLMs is model-specific and cannot be reliably approximated with coreutils. Consider using a Python/Rust tokenizer implementation instead. |
| filemap | Generate a project structure visualization | tree (not coreutils) or find + sed/awk | find . -print | sed -e 's/[^-][^\/]*/  &/g' (example)  OR  tree . | Medium — `tree` (commonly available) is closest; otherwise `find` + `awk` can build a similar view. |
| submit | Mark task as complete (agent meta) | N/A | N/A | None — agent workflow meta-action; no shell equivalent. |
| classify_task | Classify a task type for workflow routing | N/A | N/A | None — requires NLP/model. |

## Recommendations

- For simple file operations (create, copy, move, delete, search, basic edits), prefer direct coreutils commands (sed, grep, find, mv, cp, rm, mkdir, tee).
- For interactive viewing and scrolling, use less or more rather than re-implementing scroll semantics with sed.
- For complex code-aware edits (delete_function, language-aware transformations), implement or reuse language-aware parsers (Rust analyzer, tree-sitter) rather than attempting fragile regex with sed/awk.
- For token counting, use a tokenizer library that matches the LLM model (e.g., tiktoken/cl100k_base) — coreutils wc is only an approximation.

## Example quick-replacement snippets

- View lines 100–150 of a file:

```bash
sed -n '100,150p' src/main.rs
```

- Replace "foo" with "bar" in-place:

```bash
sed -i 's/foo/bar/g' path/to/file
```

- Find all .rs files:

```bash
find . -name "*.rs"
```

- Delete directory safely (review before deleting):

```bash
ls path/to/dir && rm -rf path/to/dir
```


## Notes on Safety

Many CATS tools include additional safety checks and structured outputs to make them suitable for LLM use (e.g., previewing changes, not performing destructive actions without confirmation). When replacing with shell commands, ensure you add appropriate safeguards (backups, dry-run modes, or explicit confirmation steps) to avoid data loss.

---

Generated by CATS -> GNU coreutils mapping feature.
