# OpenAI Client

This client was written to investigate the possibility of using
Azure OpenAI service with the *function call* options.

At present it does not do much with *function calls* it will
allow a conversation between a user and the AI whilst maintaining
context in a very simple manner.

## Running Tests

    cargo test -- --test-threads=1 --nocapture

## To Compile

Missing dependencies will be downloaded by cargo.

    cargo build

Build release version:

    cargo build --profile=release

## Talking to AI with Client

Each message is sent to OpenAI instance in a JSON object (see the
API documentation or the testing interface).

The application requires valid Azure authentication, these are
provided in environment variables:

```sh
export AZURE_API_KEY=
export AZURE_API_BASE=
export AZURE_API_VERSION=
```

Or:

```sh
export OAICOMPAT_API_KEY=
export OAICOMPAT_API_BASE=
export OAICOMPAT_MODEL_NAME=
```

***IMPORTANT***: `OAICOMPAT_MODEL_NAME` will be overridden by
the value of `model` in the `chat.json` file.

The history of the chat (to maintain context during a
conversation with the AI agent) is stored in a JSON file
located (by default in `chats/`) make sure this directory
exists before executing the application (though the presence
and permissions will be checked before calling the API so that
an API call is not wasted. See `openai::ChatContext::new_chat`.

    mkdir chats/

To execute the application with Cargo:

    cargo run -- --write-req-resp 0001 "What is the most efficient way to search through a sorted list?"

To execute the application directly (release version):

    target/release/openaiclient --write-req-resp 0001 "<as above>"

Chats are stored in the `chats/` directory, after the first
message the file `0001.json` should be created containing the
question and the response from GPT. The `--write-req-resp`
option will save the data sent in `last_request.json` and
`last_response.json` which will be created in the current
directory.

Then provide a follow up question from a text file:

    echo "Please provide some proof." >> followup.txt
    cargo run -- 0001 @followup.txt

Or from stdin:

    echo "Should we use a different sorting algorithm?" \
        | cargo run -- 0001 -

## Executing Commands and Getting Results

Tools must be specified in the JSON requests sent to GPT. The `datafunc/`
directory contains a possible test chat template and can be used like this:

    openaiclient --config-dir datafunc/ 1001 "How does the logic for determining the validity of the widget in rinmein.c work?"

At this point there should be a `tool_calls` array in the `assistant`
response:

    {
      "role": "assistant",
      "content": "I'll analyze the logic for determining widget validity in rinmein.c. Let me first examine the file to understand how widget validation works.\n\n",
      "tool_calls": [
        {
          "id": "82FUUHJP2cD5vMHeMMFzkyg4XrvEcT5h",
          "type": "function",
          "function": {
            "name": "read_file",
            "arguments": "{\"path\":\"rinmein.c\",\"show_line_numbers\":true}"
          }
        }
      ]
    }

To respond to the model one may send the tool response back with the `@`
functionality.

    openaiclient --role tool --name read_file 1001 @rinmein.c

The response sent back to the model will contain the content of the file
supplied:

    {
      "role": "tool",
      "content": "#include <stdio.h>\n\nint main(int argc, char *argv[]) {\n...",
      "name": "read_file",
      "tool_call_id": "82FUUHJP2cD5vMHeMMFzkyg4XrvEcT5h"
    },

The AI will use `tools` to perform tasks on the local computer, listing
files, compiling code etc. This will be facilitated by `tmux` which can
persist for more that one execution.

See ***Control Mode*** on [tmux wiki](https://github.com/tmux/tmux/wiki/Control-Mode)

## Known Working

This was compiled and working with:

    rustc 1.71.1 (eb26296b5 2023-08-03) (Alpine Linux)

Alpine Linux release 3.18.6

## References

* [openinterpreter on Azure](https://docs.openinterpreter.com/language-model-setup/hosted-models/azure)
* [litellm](https://github.com/BerriAI/litellm)
* [Error Handling](https://www.youtube.com/watch?v=UgIQo__luHw)
* [reqwest in Rust](https://www.youtube.com/watch?v=dYVJQ-KQpdc)
* [Using ChatGPT functions with Azure](https://learn.microsoft.com/en-us/azure/ai-services/openai/how-to/function-calling)
* [XDG Base Directory](https://wiki.archlinux.org/title/XDG_Base_Directory)
* [Function Calling with cURL](https://community.openai.com/t/trying-to-curl-the-response-to-a-tool-call/624919/4)

