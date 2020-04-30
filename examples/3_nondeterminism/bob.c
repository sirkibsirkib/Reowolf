#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

// bob indefinitely chooses between receiving or not receiving a message (user inputs y/n)
int main() {

	Connector* c = connector_new();
	printf("configuring...\n");
	char * pdl = buffer_pdl("eg_protocols.pdl");
	check("config ", connector_configure(c, pdl, "bob3"));
	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_native(c, 1));

	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	const unsigned char * msg;
	int i, code, msg_len;
	char yn;
	
	for (i = 0; true; i++) {
		printf("\nround %d...\n", i);
		printf("Should I receive a message? (y/n): ");
		scanf(" %c", &yn);
		if (yn == 'y') {
			printf("OK! Let's receive a message!\n");
			connector_get(c, 0);
		} else if (yn == 'n') {
			printf("OK! Let's NOT receive a message!\n");
		} else {
			printf("Expected (y/n) input!");
			continue;
		}
		check("sync ", connector_sync(c, 1000));
		if (yn == 'y') {
			check("read ", connector_gotten(c, 0, &msg, &msg_len));
			printf("Got message: `%.*s`\n", msg_len, msg);
		}
	}

	printf("cleaning up\n");
	connector_destroy(c);
	free(pdl);
	return 0;
}