import { createRouter, createWebHistory } from "vue-router";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "run", component: () => import("@/views/RunView.vue") },
    { path: "/inspect", name: "inspect", component: () => import("@/views/InspectView.vue") },
    { path: "/accounts", name: "accounts", component: () => import("@/views/AccountsView.vue") },
    { path: "/config", name: "config", component: () => import("@/views/ConfigView.vue") },
    { path: "/rules", name: "rules", component: () => import("@/views/RulesView.vue") },
  ],
});
