import React from 'react';
import { FlatList, Text } from 'react-native';

export function List({ items }) {
  return (
    <FlatList
      data={items}
      renderItem={({ item }) => <Text>{item.label}</Text>}
    />
  );
}
