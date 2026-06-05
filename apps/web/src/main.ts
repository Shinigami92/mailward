import urql from "@urql/vue";
import { createApp } from "vue";
import App from "./App.vue";
import { urqlClient } from "./lib/urql";
import { router } from "./router";
import "./style.css";

createApp(App).use(router).use(urql, urqlClient).mount("#app");
