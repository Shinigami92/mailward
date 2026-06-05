import { Client, cacheExchange, fetchExchange, subscriptionExchange } from "@urql/vue";
import { createClient as createWsClient } from "graphql-ws";

// In dev the Vite proxy forwards /graphql (HTTP + WS) to the Rust API; in prod the
// API serves the SPA, so same-origin relative URLs work in both.
const wsProtocol = location.protocol === "https:" ? "wss" : "ws";
const wsClient = createWsClient({ url: `${wsProtocol}://${location.host}/graphql/ws` });

export const urqlClient = new Client({
  url: "/graphql",
  // Always POST queries; a GET would hit the API's GraphiQL handler and return HTML.
  preferGetMethod: false,
  exchanges: [
    cacheExchange,
    fetchExchange,
    subscriptionExchange({
      forwardSubscription: (request) => ({
        subscribe: (sink) => ({
          unsubscribe: wsClient.subscribe({ ...request, query: request.query ?? "" }, sink),
        }),
      }),
    }),
  ],
});
