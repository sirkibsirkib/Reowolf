/* CBindgen generated */

#ifndef REOWOLF_HEADER_DEFINED
#define REOWOLF_HEADER_DEFINED

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Connector Connector;

typedef uint32_t ControllerId;

/**
 * Provides a binding annotation for the port with the given index with "active":
 * (The port will conenct to a "passive" port at the given address during connect())
 * Returns:
 * - 0 for success
 * - 1 if the port was already bound and was left unchanged
 * # Safety
 * TODO
 */
int connector_bind_active(Connector *connector, unsigned int proto_port_index, const char *address);

/**
 * Provides a binding annotation for the port with the given index with "native":
 * (The port is exposed for reading and writing from the application)
 * Returns:
 * # Safety
 * TODO
 */
int connector_bind_native(Connector *connector, uintptr_t proto_port_index);

/**
 * Provides a binding annotation for the port with the given index with "native":
 * (The port is exposed for reading and writing from the application)
 * Returns:
 * # Safety
 * TODO
 */
int connector_bind_passive(Connector *connector,
                           unsigned int proto_port_index,
                           const char *address);

/**
 * Configures the given Reowolf connector with a protocol description in PDL.
 * Returns:
 * # Safety
 * TODO
 */
int connector_configure(Connector *connector, char *pdl, char *main);

/**
 * Provides a binding annotation for the port with the given index with "active":
 * (The port will conenct to a "passive" port at the given address during connect())
 * Returns:
 * - 0 SUCCESS: connected successfully
 * - TODO error codes
 * # Safety
 * TODO
 */
int connector_connect(Connector *connector, uint64_t timeout_millis);

/**
 * Destroys the given connector, freeing its underlying resources.
 * # Safety
 * TODO
 */
void connector_destroy(Connector *connector);

/**
 * # Safety
 * TODO
 */
int connector_dump_log(Connector *connector);

/**
 * Resets the error message buffer.
 * Returns:
 * - 0 if an error was cleared
 * - 1 if there was no error to clear
 * # Safety
 * TODO
 */
int connector_error_clear(void);

/**
 * Returns a pointer into the error buffer for reading as a null-terminated string
 * Returns null if there is no error in the buffer.
 * # Safety
 * TODO
 */
const char *connector_error_peek(void);

/**
 * Prepares to synchronously put a message at the given port, writing it to the given buffer.
 * - 0 SUCCESS
 * - 1 this port has the wrong direction
 * - 2 this port is already marked to get
 * # Safety
 * TODO
 */
int connector_get(Connector *connector, unsigned int proto_port_index);

/**
 * # Safety
 * TODO
 */
int connector_gotten(Connector *connector,
                     unsigned int proto_port_index,
                     const unsigned char **buf_ptr_outptr,
                     unsigned int *len_outptr);

/**
 * Creates and returns Reowolf Connector structure allocated on the heap.
 */
Connector *connector_new(void);

/**
 * # Safety
 * TODO
 */
int connector_next_batch(Connector *connector);

/**
 * Prepares to synchronously put a message at the given port, reading it from the given buffer.
 * # Safety
 * TODO
 */
int connector_put(Connector *connector,
                  unsigned int proto_port_index,
                  unsigned char *buf_ptr,
                  unsigned int msg_len);

/**
 * # Safety
 * TODO
 */
int connector_sync(Connector *connector, uint64_t timeout_millis);

/**
 * Creates and returns Reowolf Connector structure allocated on the heap.
 */
Connector *connector_with_controller_id(ControllerId controller_id);

#endif /* REOWOLF_HEADER_DEFINED */
