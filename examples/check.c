void check(const char* phase, int err) {
	if (err) {
		printf("ERR %d in phase `%s`. Err was `%s`\nEXITING!\n",
			err, phase, connector_error_peek());
		exit(1);
	}
}