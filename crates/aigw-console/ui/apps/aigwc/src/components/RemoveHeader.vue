<script setup lang="ts">
import { onMounted, watch, h, computed } from 'vue'
import { Row, Col, FormItem, Input, InputGroup, Button } from 'ant-design-vue'
import { Plus, createIconifyIcon } from '@vben/icons'
const DeleteIcon = createIconifyIcon('ant-design:delete-outlined');

interface HeaderField {
    name: string
}

const props = withDefaults(defineProps<{
    min?: number
    max?: number
    namePath?: string
    label: string
}>(), {
    min: 1,
    max: 10,
})

const modelValue = defineModel<HeaderField[]>({
    required: true,
    default: () => []
});

const canAdd = computed(() => modelValue.value.length < props.max)
const canRemove = computed(() => modelValue.value.length > props.min)


const createDefaultField = (): HeaderField => ({
    name: "",
})

const addItem = () => {
    if (!canAdd.value)
        return;
    modelValue.value.push(createDefaultField());
}

const removeItem = (index: number) => {
    if (!canRemove.value)
        return;
    modelValue.value.splice(index, 1)
}

const ensureMinFields = () => {
    while (modelValue.value.length < props.min) {
        modelValue.value.push(createDefaultField());
    }
}

onMounted(() => {
    ensureMinFields()
})
watch(() => props.min, ensureMinFields)

const getFieldPath = (index: number, fieldName: string) => {
    return props.namePath + "_" + index + "_" + fieldName
}

</script>

<template>
    <div class="w-full">
        <div v-for="(item, index) in modelValue" :key="index">
            <Row>
                <Col :span="24">

                <FormItem :colon="false" :label="label" :label-col="{ span: 3 }">
                    <InputGroup compact>
                        <Input style="width: 200px;" v-model:value="item.name" placeholder="Example: Connection"
                            :name="getFieldPath(index, 'name')" />
                        <Button v-if="canAdd && index === 0" primary @click="addItem" :icon="h(Plus)" />
                        <Button v-if="canRemove" danger :icon="h(DeleteIcon)" @click="removeItem(index)" />
                    </InputGroup>
                </FormItem>

                </Col>
            </Row>
        </div>

    </div>
</template>