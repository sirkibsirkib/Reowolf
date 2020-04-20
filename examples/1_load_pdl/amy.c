#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // AMY
	char * pdl = buffer_pdl("forward.pdl");
	
	char msg_buf[128];
	memset(msg_buf, 0, 128);
	
	printf("input a message to send:");

	check("fgets", fgets(msg_buf, 128-1, stdin) == NULL);
	int msg_len = strlen(msg_buf);
	msg_buf[msg_len-1] = 0;
	printf("will send msg `%s`\n", msg_buf);
	
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_passive(c, 1, "127.0.0.1:7000"));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	int i;
	for (i = 0; i < 3; i++) {
		check("put ", connector_put(c, 0, msg_buf, msg_len));
		check("sync", connector_sync(c, 1000));
		printf("Sent one message!\n");
	}
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}