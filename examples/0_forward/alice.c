#include <stdio.h>
#include <string.h>
#include "../../reowolf.h"
#include "../check.c"

int main() { // ALICE
	
	char* pdl =
	"primitive forward(in i, out o) {"
	"	while(true) synchronous {"
	"		put(o, get(i));"
	"	}"
	"}"
	;
	
	char msg_buf[128];
	memset(msg_buf, 0, 128);
	
	printf("input a message to send:");

	check("fgets", fgets(msg_buf, 128-1, stdin) == NULL);
	int msg_len = strlen(msg_buf);
	msg_buf[msg_len-1] = 0;
	printf("sending msg `%s`\n", msg_buf);
	
	Connector* c = connector_new();
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_passive(c, 1, "127.0.0.1:7000"));
	check("connect", connector_connect(c, 10000));
	
	int i;
	for (i = 0; i < 3; i++) {
		check("put ", connector_put(c, 0, msg_buf, msg_len));
		check("sync", connector_sync(c, 10000));
		printf("SEND OK\n");
	}
	
	printf("OK\n");
	connector_destroy(c);
	return 0;
}