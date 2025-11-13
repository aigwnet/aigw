import { defineStore } from 'pinia';
import { ref } from 'vue';
export const clusterStore = defineStore('cluster', () => {

    const current = ref<string | undefined>(undefined);

    async function store(cluster: string) {
        current.value = cluster;
        // 可选：持久化到 localStorage
        localStorage.setItem('cluster', cluster);
    }

    function init() {
        const saved = localStorage.getItem('cluster');
        if (saved) {
            current.value = saved;
        }
    }

    init();

    return { store, current }
});