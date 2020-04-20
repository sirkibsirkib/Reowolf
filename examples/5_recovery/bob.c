#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // BOB!

	char * pdl = buffer_pdl("eg_protocols.pdl");
	Connector* c = connector_new();

	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "recovery_bob"));
	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_native(c, 1));

	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	int msg_len;
	const unsigned char * msg;

	int i;
	char msg_buf[1];
	int code;
	char answer;
	
	for (i = 0; true; i++) {
		printf("\nround %d...\n", i);
		printf("Should I receive a message? (y/n): ");
		scanf(" %c", &answer);
		if (answer == 'y') {
			printf("OK! Let's receive a message!\n");
			connector_get(c, 0);
		} else if (answer == 'n') {
			printf("OK! Let's NOT receive a message!\n");
		} else {
			printf("Expected (y/n) input!");
			continue;
		}
		
		code = connector_sync(c, 1000);
			
		// lets see how it went
		if (code == 0) {
			printf("Success!\n");
			if (answer == 'y') {
				check("read ", connector_gotten(c, 0, &msg, &msg_len));
				printf("Got message: `%.*s`\n", msg_len, msg);
			}
		} else if (code == -1) {
			printf("!!! UH OH! The round rolled back! Let's try again\n");
		} else {
			printf(
				"Something went wrong!\n",
				code, connector_error_peek()
			);
			break;
		}
	}
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}