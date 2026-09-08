/* Compress stdin to stdout in one deflate call at the given level, windowBits and strategy. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "zlib.h"

#define CAPACITY (64u << 20)

int main(int argc, char **argv) {
	if (argc != 4) return 2;
	int level = atoi(argv[1]);
	int window_bits = atoi(argv[2]);
	int strategy = atoi(argv[3]);
	unsigned char *in = malloc(CAPACITY), *out = malloc(CAPACITY);
	size_t n = fread(in, 1, CAPACITY, stdin);
	z_stream stream;
	memset(&stream, 0, sizeof stream);
	if (deflateInit2(&stream, level, Z_DEFLATED, window_bits, 8, strategy) != Z_OK) return 3;
	stream.next_in = in;
	stream.avail_in = n;
	stream.next_out = out;
	stream.avail_out = CAPACITY;
	if (deflate(&stream, Z_FINISH) != Z_STREAM_END) return 4;
	fwrite(out, 1, stream.total_out, stdout);
	return deflateEnd(&stream) == Z_OK ? 0 : 5;
}
