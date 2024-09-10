import { Pipe, PipeTransform } from '@angular/core';

@Pipe({
  name: 'addHyphen',
  standalone: true
})
export class AddHyphenPipe implements PipeTransform {

  transform(value: string): string {
    if (!value) {
      return '';
    }
    return value.replace(/\s+/g, '-');
  }

}