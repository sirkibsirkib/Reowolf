#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() {

	Connector* c = connector_new();
	printf("configuring...\n");
	char * pdl = buffer_pdl("eg_protocols.pdl");
	check("config ", connector_configure(c, pdl, "sync_two"));
	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_active(c, 1, "127.0.0.1:7001"));
	check("bind 2 ", connector_bind_native(c, 2));
	check("bind 3 ", connector_bind_native(c, 3));

	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	const unsigned char * msg;
	const char * nth[2] = {"first", "second"};
	int i, code, msg_len;
	char yn[2];
	
	while(true) {
		printf("\nround %d...\n", i);
		printf("Which of the two messages should we receive? (y/n)(y/n) (eg: yy): ");
		scanf(" %c%c", &yn[0], &yn[1]);
		for (i = 0; i < 2; i++) {
			if (yn[i] != 'y' && yn[i] != 'n') {
				printf("Expected (y/n) input!");
				continue;
			}
		}
		printf("Receiving messages [%c, %c]\n", yn[0], yn[1]);
		if (yn[0] == 'y') check("get first  ", connector_get(c, 0));
		if (yn[1] == 'y') check("get second ", connector_get(c, 1));
		code = connector_sync(c, 1000);
		if(code >= 0) {
			for (i = 0; i < 2; i++) {
				if (yn[i] == 'y') {
				check("read ", connector_gotten(c, i, &msg, &msg_len));
					printf("Got %s msg `%.*s`\n", nth[i], msg_len, msg);
				}
			}
		} else if(code == -1) {
			printf("No interaction! Recovered state.\n");
		} else {
			printf("Unrecoverable error!\n");
			connector_dump_log(c);
			break;
		}
	}

	printf("cleaning up\n");
	connector_destroy(c);
	free(pdl);
	return 0;
}