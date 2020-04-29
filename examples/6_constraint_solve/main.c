#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "../../reowolf.h"
#include "../utility.c"

#define N 4

typedef struct PeerInfo {
	int id;
	bool puts; // true iff the channel to this peer is INCOMING.
} PeerInfo;

// return the index of (i,j) in the lexicographic ordering of set {(i,j) : i<j, j<N}
// for convenience, swaps (i,j) if i>j 
int combination_index(int i, int j) {
	if (i > j) {
		// swap!
		i ^= j;
		j ^= i;
		i ^= j;
	}
	assert(0 <= i);
	assert(i < j);
	assert(j < N);
		
	int idx_in_square = i*N + j;
	int skipped = ((i+1) * (i+2)) / 2;
	return idx_in_square - skipped;
}

// initializes the given 3-element array,
// breaking symmetry with put-get direction.
void init_peer_infos(PeerInfo * peer_infos, int my_id) {
	int i;
	for (i = 0; i < 3; i++) {
		PeerInfo * pi = &peer_infos[i];
		pi->puts = i < my_id;
		pi->id = i < my_id ? i : i+1;
		printf("info %d puts=%d id=%d\n", i, pi->puts, pi->id);
	}
}

const char* addrs[] = {
	"127.0.0.1:7000",
	"127.0.0.1:7001",
	"127.0.0.1:7002",
	"127.0.0.1:7003",
	"127.0.0.1:7004",
	"127.0.0.1:7005",
};
int main(int arg_c, char * argv[]) {
	int my_id, peer_id, i;
	if (arg_c != 2) {
		printf("Expected one arg: which peer I am in 0..4");
		return 1;
	}
	my_id = atoi(argv[1]);
	assert(0 <= my_id && my_id < N);
	printf("I have id %d\n", my_id);
	
	char * pdl = buffer_pdl("eg_protocols.pdl");
	
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "xor_three"));
	
	PeerInfo peer_infos[3];
	init_peer_infos(peer_infos, my_id);
	
	// for every native port, bind native and protocol port.
	for (i = 0; i < 3; i++) {
		PeerInfo * pi = &peer_infos[i];
		int addr_idx = combination_index(my_id, pi->id);
		if (pi->puts) {
			check("bind to putter ",
				connector_bind_passive(c, i*2, addrs[addr_idx]));
			check("bind native ", connector_bind_native(c, i*2 + 1));
		} else {
			check("bind native ", connector_bind_native(c, i*2));
			check("bind to putter ",
				connector_bind_active(c, i*2 + 1, addrs[addr_idx]));
		}
	}
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	// for every native port, create a singleton batch
	for (i = 0; i < 3; i++) {
		if (i > 0) check("next ", connector_next_batch(c));
		PeerInfo * pi = &peer_infos[i];
		check("op ", pi->puts?
			connector_get(c, i):
			connector_put(c, i, NULL, 0));
	}
	// solve!
	int batch_idx = connector_sync(c, 3000);
	if (batch_idx < 0) printf("Error code on sync! %d\n", batch_idx);
	else printf("I was paired with peer %d\n", peer_infos[batch_idx].id);
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}