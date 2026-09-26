import { createApp } from "vue";
import App from "./App.vue";
import "./styles/theme.css";
import { initTheme } from "./theme";

initTheme();
createApp(App).mount("#app");
