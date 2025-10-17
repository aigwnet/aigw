<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { $t } from '#/locales';

import { Page, Loading } from '@vben/common-ui';
import { useTabs } from '@vben/hooks';
import { getClusterApi, updateClusterApi } from '#/api';
import { useVbenForm, z } from '#/adapter/form';
import { message, Card } from 'ant-design-vue';

const route = useRoute();

const { setTabTitle } = useTabs();

const index = computed(() => {
    return route.params?.id ?? -1;
});

setTabTitle(`${index.value} - 详情信息`);

const submitting = ref(true);

const [Form, formApi] = useVbenForm({
    handleSubmit: onSubmit,
    schema: [
        {
            component: 'Input',
            componentProps: {
                placeholder: 'Unique name',
            },
            fieldName: 'name',
            label: 'Name',
            rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
        },
        {
            component: 'Input',
            componentProps: {
                placeholder: '',
            },
            defaultValue: '',
            fieldName: 'description',
            label: 'Description',
        },
    ],
    wrapperClass: 'grid-cols-1',
    commonConfig: {
        labelWidth: 200
    },
});


const fetchData = async () => {
    const cluster = await getClusterApi(`${index.value}`);
    formApi.setValues(cluster);
    submitting.value = false;
}

async function handleAsyncSubmit(values: Record<string, any>) {
    try {
        submitting.value = true;
        await updateClusterApi(index.value, values);
        message.success({
            content: `Upadte cluster successfully!`,
        });
    } catch {

    } finally {
        submitting.value = false;
    }
}
function onSubmit(values: Record<string, any>) {
    handleAsyncSubmit(values).catch(error => {
        console.error('Submit error:', error);
    });

}


onMounted(() => {
    fetchData()
})

</script>

<template>
    <Page content-class="flex flex-col gap-4" description="" title="">
        <Card :title="$t('page.cluster.edit')">
            <Form />

            <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
                <Loading :spinning="submitting" />
            </div>
        </Card>
    </Page>
</template>