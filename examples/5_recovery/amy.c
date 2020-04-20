#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // AMY
	char * pdl = buffer_pdl("eg_protocols.pdl");
	
	Connector* c = connector_new();
	printf("configuring...\n");

	check("config ", connector_configure(c, pdl, "sync"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_passive(c, 1, "127.0.0.1:7000"));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	int i;
	int code;
	while (1) {
		printf("\nround %d. I will offer the message \"hello\".\n", i);
		connector_next_batch(c);
		check("put ", connector_put(c, 0, "hello", 5));
		code = connector_sync(c, 3000);
		
		if (code == 0) printf("OK! No message was sent!");
		else if (code == 1) printf("OK! My message was received!");
		else if (code == -1) printf("!!! UH OH! The round rolled back! Let's try again");
		else {
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