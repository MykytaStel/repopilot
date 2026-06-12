import React from 'react';
import { View, Text, StyleSheet } from 'react-native';

const styles = StyleSheet.create({
  container: { padding: 16 },
});

export function Screen() {
  return (
    <View style={styles.container}>
      <Text>Hello</Text>
    </View>
  );
}
