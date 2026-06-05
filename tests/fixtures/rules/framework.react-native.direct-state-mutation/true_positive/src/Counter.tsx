import React from 'react';
import { Text } from 'react-native';

export class Counter extends React.Component {
  state = { count: 0 };

  bump() {
    this.state.count = this.state.count + 1;
  }

  render() {
    return <Text>{this.state.count}</Text>;
  }
}
