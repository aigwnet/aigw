<script setup lang="ts">
import { ref, watch, h } from 'vue'
import { Row, Col, FormItem, Input, InputGroup, Button } from 'ant-design-vue'
import { Plus, X } from '@vben/icons'

const props = withDefaults(defineProps<{
    modelValue?: any[]
    min?: number
    max?: number
    namePath?: string
    label: string
}>(), {
    modelValue: () => [],
    min: 1,
    max: 10,
})

const localFields = ref<any[]>([...props.modelValue])

const emit = defineEmits<{
    (e: 'update:modelValue', value: any[]): void
}>()

watch(
    () => props.modelValue,
    (newVal) => {
        if (JSON.stringify(newVal) !== JSON.stringify(localFields.value)) {
            localFields.value = [...(newVal || [])]
        }
    },
    { deep: true }
)

watch(
    localFields,
    (newVal) => {
        emit('update:modelValue', newVal)
    },
    { deep: true }
)

// 添加新项
const addItem = () => {
    if (localFields.value.length >= props.max) return
    localFields.value.push({
        name: "",
        value: "",
    })
}

// 删除项
const removeItem = (index: number) => {
    if (localFields.value.length <= props.min) return
    localFields.value.splice(index, 1)
}

// 确保最小数量
const ensureMinFields = () => {
    while (localFields.value.length < props.min) {
        localFields.value.push({
            name: "",
            value: "",
        })
    }
}

// 初始化
ensureMinFields()

const getFieldPath = (index: number, fieldName: string) => {
    return props.namePath + "_" + index + "_" + fieldName
}

</script>

<template>
    <div class="w-full">
        <div v-for="(_item, index) in localFields" :key="index">
            <Row>
                <Col :span="24">

                <FormItem :label="label" :label-col="{ span: 3 }">
                    <InputGroup>
                        <Input style="width: 200px;" v-model:value="localFields[index].name"
                            placeholder="Example: Connection" :name="getFieldPath(index, 'name')" />
                        <Input style="width: 200px;" v-model:value="localFields[index].value"
                            placeholder="Example: upgrade" :name="getFieldPath(index, 'value')" />

                        <Button v-if="localFields.length < max" primary @click="addItem" :icon="h(Plus)" />
                        <Button v-if="localFields.length > min" danger :icon="h(X)" @click="removeItem(index)" />

                    </InputGroup>
                </FormItem>

                </Col>
            </Row>
        </div>

    </div>
</template>