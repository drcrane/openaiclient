#include "llama.h"

#include <iostream>
#include <vector>
#include <string>
#include <thread>
#include <cstring>

/*
 * Building with CMake:
 * cmake -S . -B build
 * cmake --build build -j 1
 * https://huggingface.co/mistralai/Ministral-3-14B-Reasoning-2512-GGUF/blob/main/Ministral-3-14B-Reasoning-2512-Q8_0.gguf
 * g++ main_generate.cpp -I../llama.cpp/include -I../llama.cpp/ggml/include -L../llama.cpp/build/bin -lggml-base -lggml -lllama -o run_generate -pthread -std=c++17
 * LD_LIBRARY_PATH=../llama.cpp/build/bin ./run_generate
 */

static std::string token_to_piece(
	const llama_vocab * vocab,
	llama_token token) {

	char buf[256];

	int n = llama_token_to_piece(
		vocab,
		token,
		buf,
		sizeof(buf),
		0,
		true);

	if (n < 0) {
		return "";
	}

	return std::string(buf, n);
}

int print_available_backends() {
	//
	// List available devices
	//

	int n_devices = ggml_backend_dev_count();

	std::cout
		<< "\n=== AVAILABLE DEVICES ===\n";

	for (int i = 0; i < n_devices; i++) {
		ggml_backend_dev_t dev =
			ggml_backend_dev_get(i);

		const char * name =
			ggml_backend_dev_name(dev);

		const char * description =
			ggml_backend_dev_description(dev);

		auto type =
			ggml_backend_dev_type(dev);

		std::string type_str;

		switch (type) {
			case GGML_BACKEND_DEVICE_TYPE_CPU:
				type_str = "CPU";
				break;

			case GGML_BACKEND_DEVICE_TYPE_GPU:
				type_str = "GPU";
				break;

			case GGML_BACKEND_DEVICE_TYPE_ACCEL:
				type_str = "ACCELERATOR";
				break;

			default:
				type_str = "UNKNOWN";
				break;
		}

		std::cout
			<< "[" << i << "] "
			<< type_str
			<< " : "
			<< name
			<< " : "
			<< description
			<< "\n";
	}
	return 0;
}

int main(int argc, char *argv[]) {
	if (argc < 3) {
		printf("Not enough arguments: %d\n", argc);
	}
	const char * model_path = argv[1];
	print_available_backends();
//	return 0;
//	llama_model_params model_params =
//	llama_model_default_params();

	//
	// Backend init
	//

	llama_backend_init();

	llama_log_set(
	[](enum ggml_log_level level,
	   const char * text,
	   void * user_data) {

		if (level >= GGML_LOG_LEVEL_ERROR) {
			fprintf(stderr, "%s", text);
		}

	},
	nullptr);

	llama_numa_init(GGML_NUMA_STRATEGY_DISABLED);

	//
	// Load model
	//

	llama_model_params model_params =
		llama_model_default_params();

	model_params.n_gpu_layers = 999;
	model_params.main_gpu = 0;

	llama_model * model =
		llama_model_load_from_file(
			model_path,
			model_params);

	if (!model) {
		std::cerr << "Failed to load model\n";
		return 1;
	}

	//
	// Context
	//

	llama_context_params ctx_params =
		llama_context_default_params();

	ctx_params.n_ctx = 2048;
	ctx_params.n_threads =
		std::thread::hardware_concurrency();
	ctx_params.n_threads_batch =
		std::thread::hardware_concurrency();

	llama_context * ctx =
		llama_init_from_model(
			model,
			ctx_params);

	if (!ctx) {
		std::cerr << "Failed to create context\n";
		return 1;
	}

	//
	// Vocab
	//

	const llama_vocab * vocab =
		llama_model_get_vocab(model);

	//
	// Prompt
	// This Prompt contains special tokens for the Mistral AI models.
	// Qwen, Gemma, DeepSeek and OpenAI Models all have their own "reserved words"
	//

	std::string prompt_ministral =
		"<s>[INST]Hello, how are you today?[/INST]";

	std::string prompt_qwen =
		"<|im_start|>system\nYou are a helpful assistant<|im_end|>\n<|im_start|>user\nHello, how are you today?<|endoftext|>\n<think>";

	std::string prompt = prompt_ministral;

	//
	// Tokenize
	//

	std::vector<llama_token> tokens(512);

	int n_tokens = llama_tokenize(
		vocab,
		prompt.c_str(),
		prompt.size(),
		tokens.data(),
		tokens.size(),
		false,   // add_special
		true);  // parse_special

	if (n_tokens < 0) {
		std::cerr << "Tokenization failed\n";
		return 1;
	}

	tokens.resize(n_tokens);

	//
	// Print token IDs
	//

	std::cout << "\n=== TOKEN IDS ===\n";

	for (auto t : tokens) {
		std::cout << t << "\n";
	}

	//
	// Print token text
	//

	std::cout << "\n=== TOKEN TEXT ===\n";

	for (auto t : tokens) {
		std::string piece =
			token_to_piece(vocab, t);

		std::cout
			<< "[" << t << "] "
			<< piece
			<< "\n";
	}

	//
	// Create initial batch
	//

	llama_batch batch =
		llama_batch_init(tokens.size(), 0, 1);

	for (size_t i = 0; i < tokens.size(); i++) {
		batch.token[i] = tokens[i];
		batch.pos[i] = i;

		batch.n_seq_id[i] = 1;
		batch.seq_id[i][0] = 0;

		//
		// only final token needs logits
		//

		batch.logits[i] = false;
	}

	//
	// Request logits for final token
	//

	batch.logits[tokens.size() - 1] = true;

	batch.n_tokens = tokens.size();

	//
	// Decode prompt
	//

	if (llama_decode(ctx, batch) != 0) {
		std::cerr << "Decode failed\n";

		llama_batch_free(batch);

		llama_free(ctx);
		llama_model_free(model);
		llama_backend_free();

		return 1;
	}

	//
	// Generation
	//

	std::cout << "\n=== GENERATION ===\n";

	for (int i = 0; i < 64; i++) {

		//
		// Get logits from final position
		//

		const float * logits =
			llama_get_logits_ith(
				ctx,
				batch.n_tokens - 1);

		if (!logits) {
			std::cerr << "Failed to get logits\n";
			break;
		}

		//
		// Greedy sampling
		//

		llama_token new_token_id =
			llama_vocab_eos(vocab);

		float best_logit = -1e9f;

		int n_vocab =
			llama_vocab_n_tokens(vocab);

		for (llama_token token_id = 0;
			 token_id < n_vocab;
			 token_id++) {

			float logit = logits[token_id];

			if (logit > best_logit) {
				best_logit = logit;
				new_token_id = token_id;
			}
		}

		//
		// Stop on EOS
		//

		if (new_token_id ==
			llama_vocab_eos(vocab)) {

			std::cout << "\n[EOS]\n";
			break;
		}

		//
		// Convert token to text
		//

		std::string piece =
			token_to_piece(
				vocab,
				new_token_id);

		//
		// Stream text output
		//

		std::cout
			<< piece
			<< std::flush;

		//
		// Debug token info
		//
/*
		std::cout
			<< "\n[TOKEN "
			<< new_token_id
			<< "] "
			<< piece
			<< "\n";
*/
		//
		// Free previous batch
		//

		llama_batch_free(batch);

		//
		// Create next token batch
		//

		batch = llama_batch_init(1, 0, 1);

		batch.token[0] = new_token_id;

		//
		// position in sequence
		//

		batch.pos[0] = tokens.size();

		batch.n_seq_id[0] = 1;
		batch.seq_id[0][0] = 0;

		//
		// request logits again
		//

		batch.logits[0] = true;

		batch.n_tokens = 1;

		//
		// Decode new token
		//

		if (llama_decode(ctx, batch) != 0) {
			std::cerr << "Decode failed\n";
			break;
		}

		//
		// Keep token history
		//

		tokens.push_back(new_token_id);
	}

	//
	// Cleanup
	//

	llama_batch_free(batch);

	llama_free(ctx);

	llama_model_free(model);

	llama_backend_free();

	return 0;
}
