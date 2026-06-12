import React from 'react';
import { FlatList, Text } from 'react-native';

export function List({ items }) {
  return (
    <FlatList
      data={items}
      keyExtractor={(item) => item.id.toString()}
      renderItem={({ item }) => <Text>{item.label}</Text>}
    />
  );
}
