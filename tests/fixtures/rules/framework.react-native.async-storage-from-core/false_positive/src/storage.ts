import AsyncStorage from '@react-native-async-storage/async-storage';

export async function saveToken(token: string) {
  await AsyncStorage.setItem('token', token);
}
