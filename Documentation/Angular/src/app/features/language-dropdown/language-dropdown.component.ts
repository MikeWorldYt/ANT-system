import { Component } from '@angular/core';
import { LanguageService } from '../../services/navLanguage.service';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-language-dropdown',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './language-dropdown.component.html',
  styleUrl: './language-dropdown.component.css'
})
export class LanguageDropdownComponent {
  languages: string[] = ['EN', 'ES']; // Idiomas disponibles

  constructor(private languageService: LanguageService) {}

  changeLanguage(event: Event) {
    const selectElement = event.target as HTMLSelectElement;
    const selectedLanguage = selectElement.value as string;
    this.languageService.setLanguage(selectedLanguage);
  }
}
