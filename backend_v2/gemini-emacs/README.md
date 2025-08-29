# Gemini Code

Emacs interfaces for Google's Gemini models, inspired by `claude-code.el`.

This package provides three ways to interact with Gemini:
1. **Simple API mode** (`gemini-code.el`) - Direct API calls with response in buffer
2. **Terminal mode** (`gemini-native.el`) - Full terminal interface using native Gemini CLI
3. **Custom CLI mode** (`gemini-code-term.el`) - Terminal interface with custom Python CLI

## Features

*   Send prompts to Gemini from within Emacs.
*   Send the content of the current region or buffer to Gemini.
*   View Gemini's responses in a dedicated Emacs buffer.
*   Support for multiple, named Gemini instances.
*   Direct integration with the Gemini API (no external CLI needed).

## Installation

### 1. API Key Setup

You need to set your Gemini API key as an environment variable. Add the following line to your shell's configuration file (e.g., `~/.bashrc`, `~/.zshrc`):

```bash
export GEMINI_API_KEY="YOUR_API_KEY"
```

Replace `"YOUR_API_KEY"` with your actual Gemini API key.

### 2. Emacs Configuration

Add the `elisp` directory to your Emacs `load-path` and load the `gemini-code` library. Here's an example of how to do that in your `init.el`:

```elisp
(add-to-list 'load-path "/path/to/gemini-emacs/elisp")
(require 'gemini-code)
```

Replace `/path/to/gemini-emacs/` with the actual path to the `gemini-emacs` directory.

That's it! There are no other dependencies.

## Usage

All commands are available under the `C-c g` prefix.

### Multi-Instance Model

This tool supports multiple Gemini instances. Each instance is a separate buffer.

*   When you are in a normal buffer (e.g., editing a file), commands will use the default `*gemini*` buffer.
*   When you are inside a Gemini buffer (e.g., `*gemini-my-session*`), commands will use that buffer.

### Keybindings

*   `C-c g i`: Create a new, named Gemini instance. You will be prompted for a name, and a new buffer like `*gemini-your-name*` will be created.
*   `C-c g s`: Send a prompt to Gemini.
*   `C-c g r`: Send the selected region to Gemini.
*   `C-c g x`: Send a prompt with the current file name and line number as context.
*   `C-c g t`: Show or hide the current Gemini buffer.
*   `C-c g b`: Switch to the current Gemini buffer.
*   `C-c g c`: Start a new session (clears the current Gemini buffer).
*   `C-c g k`: Kill the current Gemini session (kills the current Gemini buffer).